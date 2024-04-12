FROM rust:bookworm as builder

ARG GIT_COMMIT
RUN test -n "$GIT_COMMIT" || (echo "GIT_COMMIT not set" && false)

RUN echo "Building stacks-node from commit: https://github.com/stacks-network/stacks-blockchain/commit/$GIT_COMMIT"

RUN apt-get update && apt-get install -y libclang-dev
RUN rustup toolchain install stable
RUN rustup component add rustfmt --toolchain stable

WORKDIR /stacks
RUN git init && \
    git remote add origin https://github.com/stacks-network/stacks-blockchain.git && \
    git -c protocol.version=2 fetch --depth=1 origin "$GIT_COMMIT" && \
    git reset --hard FETCH_HEAD

RUN cargo build --release --package stacks-node --package stacks-signer --bin stacks-node --bin stacks-signer

FROM debian:bookworm-slim

COPY --from=builder /stacks/target/release/stacks-node /bin
COPY --from=builder /stacks/target/release/stacks-signer /bin

RUN apt update
RUN apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

WORKDIR /root
