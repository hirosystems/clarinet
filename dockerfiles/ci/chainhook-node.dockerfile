FROM debian:bullseye-slim

COPY chainhook-node /bin/chainhook-node

RUN apt update && apt install -y libssl-dev

WORKDIR /workspace

ENTRYPOINT ["chainhook-node"]
