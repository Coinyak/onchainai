# OnchainAI — multi-stage Docker build.
# Builder: rust:1.85-slim, runtime: debian:bookworm-slim.

FROM rust:1.85-slim AS builder
WORKDIR /app

# Install build dependencies needed by some crates (e.g. openssl, pkg-config).
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better layer caching.
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY style/ ./style/

# Build release binary.
RUN cargo build --release

# --- runtime stage ---
FROM debian:bookworm-slim
WORKDIR /app

# Runtime libs needed by the binary.
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/onchainai /app/onchainai
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/style /app/style

ENV PORT=3000
ENV RUST_LOG=info
EXPOSE 3000

CMD ["/app/onchainai"]
