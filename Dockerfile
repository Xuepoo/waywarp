# Multi-stage Dockerfile for Rust CLI projects
# Produces minimal runtime images for both amd64 and arm64

# --- Build stage ---
FROM rust:latest AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    libwayland-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libxkbcommon-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy source and build
COPY . .
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libwayland-client0 \
    libcairo2 \
    libpango-1.0-0 \
    libpangocairo-1.0-0 \
    libxkbcommon0 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/waywarp /usr/local/bin/waywarp

# Run as non-root user
RUN useradd --create-home appuser
USER appuser

ENTRYPOINT ["waywarp"]
