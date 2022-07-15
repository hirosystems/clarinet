FROM alpine
ARG BTC_URL="https://github.com/hirosystems/bitcoin-docker/releases/download/0.21.1/musl-v0.21.1.tar.gz"
WORKDIR /
RUN apk add --update \
    curl \
    gnupg \
    boost-system \
    boost-filesystem \
    boost-thread \
    boost-chrono \
    libevent \
    libzmq \
    libgcc \
    tini \
    jq
RUN curl -L -o /bitcoin.tar.gz ${BTC_URL}
RUN tar -xzvf /bitcoin.tar.gz
RUN mkdir -p /root/.bitcoin
RUN mv /bitcoin-*/bin/* /usr/local/bin/
RUN rm -rf /bitcoin-*
ENTRYPOINT ["/sbin/tini", "--"]
CMD [ "/usr/local/bin/bitcoind -conf=/etc/bitcoin/bitcoin.conf -nodebuglogfile -pid=/run/bitcoind.pid -datadir=/root/.bitcoin"]
