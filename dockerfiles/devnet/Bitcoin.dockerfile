FROM alpine as build

WORKDIR /src

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
    libevent \
    libzmq \
    libgcc \
    sqlite \
    sqlite-libs \
    && mkdir /bitcoin

COPY --from=build /out/ /usr/local/bin/
