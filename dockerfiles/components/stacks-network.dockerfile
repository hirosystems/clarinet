FROM rust:bullseye as build

COPY . ./

RUN cargo build --release --manifest-path ./components/stacks-network/Cargo.toml

# prod stage
FROM debian:bullseye-slim
COPY --from=build target/release/stacks-network /

ENTRYPOINT ["./stacks-network"]