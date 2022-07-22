FROM debian:bullseye-slim

COPY clarinet /bin/clarinet

RUN apt update && apt install -y libssl-dev

WORKDIR /workspace

ENV CLARINET_MODE_CI=1

ENTRYPOINT ["clarinet"]
