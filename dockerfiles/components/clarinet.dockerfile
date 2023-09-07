FROM debian:bookworm-slim

RUN apt update && apt install -y libssl-dev

RUN rustup update 1.71.0 && rustup default 1.71.0

COPY clarinet /bin/

WORKDIR /workspace

ENV CLARINET_MODE_CI=1

ENTRYPOINT ["clarinet"]
