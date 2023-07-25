FROM rust:bookworm as build

WORKDIR /src

RUN apt update && apt install -y ca-certificates pkg-config libssl-dev

RUN rustup update 1.67.0 && rustup default 1.67.0

COPY . .

RUN mkdir /out

RUN cargo build --features=telemetry --release --locked

RUN cp target/release/clarinet /out

FROM debian:bookworm-slim

RUN apt update && apt install -y libssl-dev

COPY --from=build /out/ /bin/

WORKDIR /workspace

ENV CLARINET_MODE_CI=1

ENTRYPOINT ["clarinet"]
