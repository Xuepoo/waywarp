# Multi-stage Dockerfile for Rust CLI projects
# Produces minimal runtime images for both amd64 and arm64

# --- Build stage ---
FROM rust:latest AS builder

WORKDIR /build

# Cache dependencies by building with empty main first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release && rm -rf src

# Build the actual binary
COPY . .
RUN touch src/main.rs && cargo build --release --locked

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/waywarp /usr/local/bin/waywarp

# Run as non-root user
RUN useradd --create-home appuser
USER appuser

ENTRYPOINT ["waywarp"]
