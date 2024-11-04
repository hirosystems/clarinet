FROM rust:bookworm as builder

ARG GIT_COMMIT
RUN test -n "$GIT_COMMIT" || (echo "GIT_COMMIT not set" && false)

RUN echo "Building stacks-node from commit: https://github.com/hugocaillard/stacks-core/commit/$GIT_COMMIT"

WORKDIR /stacks
RUN git init && \
<<<<<<< HEAD
    git remote add origin https://github.com/stacks-network/stacks-blockchain.git && \
    git fetch --depth=1 origin "$GIT_COMMIT" && \
=======
    git remote add origin https://github.com/hugocaillard/stacks-core.git && \
    git -c protocol.version=2 fetch --depth=1 origin "$GIT_COMMIT" && \
>>>>>>> 452c0840 (feat: initial implementation of mainnet execution simulation)
    git reset --hard FETCH_HEAD

RUN cargo build --package stacks-node --bin stacks-node --features monitoring_prom,slog_json --release

FROM debian:bookworm-slim

COPY --from=builder /stacks/target/release/stacks-node /bin

RUN apt update
RUN apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

WORKDIR /root
