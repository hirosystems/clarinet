FROM rust:stretch as build

WORKDIR /src

RUN apt update && apt install -y ca-certificates pkg-config libssl-dev

RUN rustup update 1.59.0 && rustup default 1.59.0

COPY . .

RUN mkdir /out

RUN cargo build --release --locked

RUN cp target/release/orchestra-event-observer /out

FROM debian:stretch-slim

RUN apt update && apt install -y libssl-dev

COPY --from=build /out/ /bin/

WORKDIR /workspace

ENTRYPOINT ["orchestra-event-observer"]
