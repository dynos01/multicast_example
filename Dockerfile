FROM alpine:latest

ADD target/x86_64-unknown-linux-musl/release/multicast_example /usr/bin/
RUN chmod +x /usr/bin/multicast_example

ENV NODE_NAME=""
ENTRYPOINT /usr/bin/multicast_example $NODE_NAME
