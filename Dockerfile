FROM rust:alpine AS build

# Set up build environment
COPY . /build
WORKDIR /build

# Compile data
RUN apk add --no-cache python3 py3-pip py3-pillow
RUN python scripts/compile_data.py

# Build the server application
RUN cargo build --release

FROM alpine

COPY --from=build /build/target/release/mydex /opt/mydex/
COPY --from=build /build/compiled_data /opt/mydex/
COPY --from=build /build/templates /opt/mydex/
ENTRYPOINT ["/opt/mydex/mydex"]
