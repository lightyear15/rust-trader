FROM rust:slim-bullseye as builder
RUN apt-get update
RUN apt-get install --yes libssl-dev perl pkg-config

WORKDIR /root/trader
COPY . .
RUN cargo install --path .

FROM debian:stable-slim
RUN apt-get update
RUN apt-get install --yes ca-certificates
COPY --from=builder /root/trader/target/release/trader /usr/bin
RUN chmod +x /usr/bin/trader
VOLUME ["/root"]
WORKDIR /root
ENTRYPOINT ["/usr/bin/trader"]
