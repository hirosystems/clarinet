FROM rust:stretch as build

ARG STACKS_NODE_VERSION="No Version Info"
ARG GIT_BRANCH='No Branch Info'
ARG GIT_COMMIT='No Commit Info'

WORKDIR /src

COPY . .

RUN mkdir /out

RUN apt update && apt install ca-certificates

RUN rustup update stable

RUN rustup default stable-aarch64-unknown-linux-gnu

RUN cargo build --release --locked

RUN cp target/release/clarinet /out

FROM debian:stretch-slim

COPY --from=build /out/ /bin/

ENTRYPOINT ["clarinet"]