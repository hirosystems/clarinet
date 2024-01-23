FROM rust:bookworm as build

WORKDIR /src

RUN apt update && apt install -y ca-certificates pkg-config libssl-dev libclang-dev

RUN rustup update stable && rustup default stable && rustup toolchain install stable --component rustfmt

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
