use std::{
    env,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    process::exit,
};

use tokio::net::UdpSocket;
use futures::{stream::FuturesUnordered, StreamExt};
use anyhow::{Result, Error};
use once_cell::sync::OnceCell;
use serde::{Serialize, Deserialize};
use pnet::datalink;
use cidr_utils::cidr::Ipv6Cidr;

const IPV4_MULTICAST_ADDR: &'static str = "224.0.0.114";
const IPV6_MULTICAST_ADDR: &'static str = "ff12:114:514:1919::810";
const PORT: u16 = 5679;

static THIS_NODE: OnceCell<Node> = OnceCell::new();

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq)]
struct Node {
    pub name: String,
}

async fn start_listener(ip: IpAddr) -> Result<()> {
    let socket = SocketAddr::new(ip, PORT);
    let socket = UdpSocket::bind(socket).await?;
    let multicast_ipv4_addr: Ipv4Addr = IPV4_MULTICAST_ADDR.parse()?;
    let multicast_ipv6_addr: Ipv6Addr = IPV6_MULTICAST_ADDR.parse()?;
    socket.join_multicast_v4(multicast_ipv4_addr, Ipv4Addr::UNSPECIFIED)?;
    socket.join_multicast_v6(&multicast_ipv6_addr, 0)?;

    let mut buf = [0u8; 1024];

    loop {
        let (len, src) = socket.recv_from(&mut buf).await?;

        let node: Node = match serde_json::from_slice(&buf[..len]) {
            Ok(node) => node,
            Err(e) => {
                eprintln!("Failed to parse message from {src}: {e}. ");
                continue;
            },
        };

        println!("Got peer {} from {src}. ", node.name);

        let response = serde_json::to_vec(THIS_NODE.wait())?;
        socket.send_to(&response, src).await?;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} username interface", args[0]);
        exit(1);
    }

    THIS_NODE.get_or_init(|| Node {
        name: args[1].clone(),
    });

    let mut futures = FuturesUnordered::new();

    futures.push(tokio::spawn(async move {
        start_listener("::".parse::<IpAddr>()?).await?;
        Ok::<(), Error>(())
    }));

    for interface in datalink::interfaces() {
        if interface.name != args[2] {
            continue;
        }

        for ip in interface.ips {
            let ip = ip.ip();

            match ip {
                IpAddr::V4(ipv4) => futures.push(tokio::spawn(async move {
                    let socket: SocketAddr = format!("{ipv4}:0").parse()?;
                    let socket = UdpSocket::bind(socket).await?;

                    let remote: SocketAddr = format!("{IPV4_MULTICAST_ADDR}:{PORT}").parse()?;
                    socket.set_multicast_loop_v4(false)?;

                    let message = serde_json::to_vec(THIS_NODE.wait())?;
                    socket.send_to(&message, remote).await?;

                    let mut buf = [0u8; 1024];

                    loop {
                        let (len, remote) = socket.recv_from(&mut buf).await?;
                        let peer: Node = serde_json::from_slice(&buf[..len])?;
                        println!("Got peer {} from {remote}. ", peer.name);
                    }
                })),
                IpAddr::V6(ipv6) => futures.push(tokio::spawn(async move {
                    let link_local = Ipv6Cidr::from_str("fe80::/10")?;
                    if link_local.contains(ipv6) {
                        return Ok(());
                    }

                    let socket: SocketAddr = format!("[{ipv6}]:0").parse()?;
                    let socket = UdpSocket::bind(socket).await?;

                    let remote: SocketAddr = format!("[{IPV6_MULTICAST_ADDR}]:{PORT}").parse()?;
                    socket.set_multicast_loop_v6(false)?;

                    let message = serde_json::to_vec(THIS_NODE.wait())?;
                    socket.send_to(&message, remote).await?;

                    let mut buf = [0u8; 1024];

                    loop {
                        let (len, remote) = socket.recv_from(&mut buf).await?;
                        let peer: Node = serde_json::from_slice(&buf[..len])?;
                        println!("Got peer {} from {remote}. ", peer.name);
                    }
                })),
            };
        }
    }

    while let Some(future) = futures.next().await {
        future??;
    }

    Ok(())
}
