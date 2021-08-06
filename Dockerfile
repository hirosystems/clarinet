FROM rust:stretch as build

WORKDIR /src

RUN apt update && apt install -y ca-certificates pkg-config libssl-dev

RUN rustup update nightly-2021-08-05 && rustup default nightly-2021-08-05

COPY . .

RUN mkdir /out

RUN cargo build --release --locked

RUN cp target/release/clarinet /out

FROM debian:stretch-slim

RUN apt update && apt install -y libssl-dev

COPY --from=build /out/ /bin/

WORKDIR /workspace

ENTRYPOINT ["clarinet"]
