FROM rust:bullseye as build

WORKDIR /src

RUN apt update && apt install -y ca-certificates pkg-config libssl-dev

RUN rustup update 1.59.0 && rustup default 1.59.0

COPY ./components/chainhook-types-rs /src/components/chainhook-types-rs

COPY ./components/chainhook-event-observer /src/components/chainhook-event-observer

COPY ./components/stacks-rpc-client /src/components/stacks-rpc-client

COPY ./components/clarity-repl /src/components/clarity-repl

COPY ./components/clarinet-utils /src/components/clarinet-utils

COPY ./components/hiro-system-kit /src/components/hiro-system-kit

WORKDIR /src/components/chainhook-event-observer

RUN mkdir /out

RUN cargo build --release

RUN cp target/release/chainhook-event-observer /out

FROM debian:bullseye-slim

RUN apt update && apt install -y libssl-dev

COPY --from=build /out/ /bin/

WORKDIR /workspace

ENTRYPOINT ["chainhook-event-observer"]