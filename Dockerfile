FROM rust:alpine AS build

COPY . /build
WORKDIR /build
RUN cargo build --release

FROM alpine

COPY --from=build /build/target/release/mydex /usr/local/bin/
ENTRYPOINT /usr/local/bin/mydex
