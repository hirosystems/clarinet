FROM alpine as build

ARG BTC_VERSION="24.0"
ARG BDB_PREFIX="/src/bitcoin/db4"

ENV BTC_VERSION=${BTC_VERSION}
ENV BDB_PREFIX=${BDB_PREFIX}

WORKDIR /src
RUN apk --no-cache add --update \
    libgcc \
    boost-dev \
    boost-thread \
    boost-filesystem \
    boost-system \
    openssl \
    autoconf \
    libtool \
    pkgconf \
    pkgconf-dev \
    libevent \
    git \
    czmq-dev \
    libzmq \
    gcc \
    g++ \
    openssl-dev \
    libevent-dev \
    make \
    automake \
    musl-dev \
    linux-headers \
    libc-dev \
    db-c++ \
    patch \
    sqlite-dev \
    sqlite \
    && /sbin/ldconfig /usr/lib /lib \
    && mkdir /out


RUN git clone --depth 1 --branch v${BTC_VERSION} https://github.com/bitcoin/bitcoin \
    && cd bitcoin \
    && sh contrib/install_db4.sh . \
    && ./autogen.sh \
    && ./configure \
        BDB_LIBS="-L${BDB_PREFIX}/lib -ldb_cxx-4.8" \
        BDB_CFLAGS="-I${BDB_PREFIX}/include"  \
        --disable-tests  \
        --enable-static  \
        --without-miniupnpc  \
        --with-pic  \
        --enable-cxx  \
        --with-sqlite=yes  \
        --with-gui=no  \
        --enable-util-util=no  \
        --enable-util-tx=no  \
        --with-boost-libdir=/usr/lib \
        --bindir=/out \
    && make -j2 STATIC=1 \
    && make install \
    && strip /out/*

FROM alpine
RUN apk --no-cache add --update \
    curl \
    boost-system \
    boost-filesystem \
    boost-thread \
    boost-chrono \
    libevent \
    libzmq \
    libgcc \
    sqlite \
    sqlite-libs \
    && mkdir /bitcoin
COPY --from=build /out/ /bin/

CMD ["/bin/bitcoind", "-server", "-datadir=/bitcoin", "-rpcuser=btcuser", "-rpcpassword=btcpass", "-rpcallowip=0.0.0.0/0", "-bind=0.0.0.0:8333", "-rpcbind=0.0.0.0:8332", "-dbcache=512", "-rpcthreads=256", "-disablewallet", "-txindex"]