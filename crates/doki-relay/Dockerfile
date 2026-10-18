# Build from the repository root:
#   docker build -f crates/doki-relay/Dockerfile -t doki-relay .
FROM rust:1.89-slim AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p doki-relay

FROM debian:bookworm-slim
RUN useradd --system --home /data doki && mkdir -p /data && chown doki /data
COPY --from=build /src/target/release/doki-relay /usr/local/bin/doki-relay
USER doki
VOLUME /data
EXPOSE 8787
ENTRYPOINT ["doki-relay", "--data", "/data", "--listen", "0.0.0.0:8787"]
