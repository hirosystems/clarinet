# build stage
FROM arm64v8/rust:1.67 as builder

COPY . ./

RUN cargo build --release --manifest-path ./components/stacks-network/Cargo.toml

# prod stage
FROM gcr.io/distroless/cc
COPY --from=builder target/release/stacks-network /

ENTRYPOINT ["./stacks-network"]