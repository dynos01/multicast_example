# Multicast Local Peer Discovery Demo
This is a demo showing how to discover peers located on the same LAN using multicast. Both IPv4 and IPv6 are supported.

By default, the program uses `224.0.0.114:5679` for IPv4 multicast and `[ff12:114:514:1919::810]:5679` for IPv6 multicast. Discovered peers are simply printed.

It is possible to use Docker to perform experiments. Remember to enable IPv6 for Docker according to steps listed [here](https://docs.docker.com/config/daemon/ipv6/).
