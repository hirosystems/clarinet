FROM rust:stretch as build

WORKDIR /src

RUN apt update && apt install -y ca-certificates pkg-config libssl-dev

RUN rustup update 1.52.1 && rustup default 1.52.1

COPY . .

RUN mkdir /out

RUN cargo build --release --locked

RUN cp target/release/clarinet /out

FROM debian:stretch-slim

RUN apt update && apt install -y libssl-dev

COPY --from=build /out/ /bin/

WORKDIR /workspace

ENTRYPOINT ["clarinet"]
