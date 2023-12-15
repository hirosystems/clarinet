FROM debian as build

ARG BTC_VERSION="25.0"
ARG BDB_PREFIX="/src/bitcoin/db4"

ENV BTC_VERSION=${BTC_VERSION}
ENV BDB_PREFIX=${BDB_PREFIX}

WORKDIR /src
RUN apt-get update && apt-get install -y \
    autoconf \
    automake \
    autotools-dev \
    bsdmainutils \
    build-essential \
    clang \
    curl \
    git \
    libboost-dev \
    libboost-filesystem-dev \
    libboost-system-dev \
    libboost-thread-dev \
    libczmq-dev \
    libevent-dev \
    libminiupnpc-dev \
    libnatpmp-dev \
    libsqlite3-dev \
    libssl-dev \
    libtool \
    pkg-config \
    python3 \
    wget \
    && /sbin/ldconfig /usr/lib /lib \
    && mkdir /out

COPY . .


RUN git clone --depth 1 --branch v${BTC_VERSION} https://github.com/bitcoin/bitcoin \
    && cd bitcoin \
    && make -C depends NO_BOOST=1 NO_LIBEVENT=1 NO_QT=1 NO_SQLITE=1 NO_NATPMP=1 NO_UPNP=1 NO_ZMQ=1 NO_USDT=1 \
    && ./autogen.sh \
    && export BDB_PREFIX="$(ls -d $(pwd)/depends/* | grep "linux-gnu")" \
    && export CXXFLAGS="-O2" \
    && echo "BDB_PREFIX: ${BDB_PREFIX}" \
    && ./configure \
        CXX=clang++ \
        CC=clang \
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
        --includedir=${BIN_DIR}/include \
        --bindir=${BIN_DIR}/bin \
        --mandir=${BIN_DIR}/share/man/man1  \
        --disable-gui-tests  \
        --disable-shared  \
        --bindir=/out \
    && make -j2 STATIC=1 \
    && make install \
    && strip /out/*

FROM debian
RUN apt-get update && apt-get install -y \
    autoconf \
    automake \
    autotools-dev \
    bsdmainutils \
    build-essential \
    clang \
    curl \
    git \
    libboost-dev \
    libboost-filesystem-dev \
    libboost-system-dev \
    libboost-thread-dev \
    libczmq-dev \
    libevent-dev \
    libminiupnpc-dev \
    libnatpmp-dev \
    libsqlite3-dev \
    libssl-dev \
    libtool \
    pkg-config \
    python3 \
    wget \
    && mkdir /bitcoin
COPY --from=build /out/ /usr/local/bin/

CMD ["/usr/local/bin/bitcoind", "-server", "-datadir=/bitcoin", "-rpcuser=btcuser", "-rpcpassword=btcpass", "-rpcallowip=0.0.0.0/0", "-bind=0.0.0.0:8333", "-rpcbind=0.0.0.0:8332", "-dbcache=512", "-rpcthreads=256", "-disablewallet", "-txindex"]
