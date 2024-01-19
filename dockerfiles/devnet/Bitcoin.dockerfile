FROM alpine as build

RUN apk --no-cache add --update \
    libgcc \
    boost-dev \
    curl \
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

WORKDIR /src

RUN wget https://github.com/bitcoin/bitcoin/archive/refs/tags/v26.0.tar.gz && tar -xvf v26.0.tar.gz

RUN cd bitcoin-26.0 \
    && ./autogen.sh \
    && export CXXFLAGS="-O2" \
    && ./configure \
        CXX=g++ \
        CC=gcc \
        --disable-gui-tests \
        --disable-tests \
        --without-miniupnpc \
        --with-pic \
        --enable-cxx \
        --enable-static \
        --disable-shared \
    && make -j2 STATIC=1 \
    && make install

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

CMD ["bitcoind", "-server", "-datadir=/bitcoin", "-rpcuser=btcuser", "-rpcpassword=btcpass", "-rpcallowip=0.0.0.0/0", "-bind=0.0.0.0:8333", "-rpcbind=0.0.0.0:8332", "-dbcache=512", "-rpcthreads=256", "-disablewallet", "-txindex"]
