FROM rust:bullseye as build

WORKDIR /src

RUN apt update && apt install -y ca-certificates pkg-config libssl-dev

RUN rustup update 1.59.0 && rustup default 1.59.0

COPY ./vendor/orchestra-types-rs /src/components/orchestra-types-rs

COPY ./vendor/orchestra-event-observer /src/components/orchestra-event-observer

COPY ./components/stacks-rpc-client /src/components/stacks-rpc-client

COPY ./components/clarity-repl /src/components/clarity-repl

WORKDIR /src/components/orchestra-event-observer

RUN mkdir /out

RUN cargo build --release

RUN cp target/release/orchestra-event-observer /out

FROM debian:bullseye-slim

RUN apt update && apt install -y libssl-dev

COPY --from=build /out/ /bin/

WORKDIR /workspace

ENTRYPOINT ["orchestra-event-observer"]