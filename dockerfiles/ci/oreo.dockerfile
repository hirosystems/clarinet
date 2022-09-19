FROM debian:bullseye-slim

COPY chainhook-event-observer /bin/chainhook-event-observer

RUN apt update && apt install -y libssl-dev

WORKDIR /workspace

ENTRYPOINT ["chainhook-event-observer"]
