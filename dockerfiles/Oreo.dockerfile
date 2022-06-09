FROM rust:stretch as build

WORKDIR /src

RUN apt update && apt install -y ca-certificates pkg-config libssl-dev protobuf-compiler

RUN rustup update 1.59.0 && rustup default 1.59.0

COPY ./orchestra-types-rs /src/orchestra-types-rs

COPY ./orchestra-event-observer /src/orchestra-event-observer

COPY ./stacks-rpc-client /src/stacks-rpc-client

WORKDIR /src/orchestra-event-observer

RUN mkdir /out

ENV PROTOC=/usr/bin/protoc

ENV PROTOC_INCLUDE=/usr/include

RUN cargo build --release

RUN cp target/release/orchestra-event-observer /out

FROM debian:stretch-slim

RUN apt update && apt install -y libssl-dev

COPY --from=build /out/ /bin/

WORKDIR /workspace

ENTRYPOINT ["orchestra-event-observer"]
