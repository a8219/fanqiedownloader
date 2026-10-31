FROM rust:1.82-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY src src
COPY static static
RUN cargo build --release --locked || cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/noveldownloader /usr/local/bin/noveldownloader
ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
USER 65534:65534
ENTRYPOINT ["/usr/local/bin/noveldownloader"]
