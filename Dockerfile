# OnchainAI — multi-stage Docker build (SSR server + WASM hydration bundle).
# Cache-bust: 2026-07-02T00:00Z
#
# Optimized for Railway builds:
# - Dependency layer cached separately from source (Cargo.toml/Cargo.lock first)
# - BuildKit cache mounts for cargo registry/git and target dir
# - HEALTHCHECK so Railway routes traffic as soon as the app is ready

# syntax=docker/dockerfile:1.7
FROM rust:1.90-slim AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential pkg-config libssl-dev curl perl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# Install cargo-leptos (portable across amd64/arm64 builders)
ARG CARGO_LEPTOS_VERSION=0.3.6
RUN cargo install cargo-leptos --version "${CARGO_LEPTOS_VERSION}" --locked

# --- Dependency caching layer ---
# Copy only manifests so cargo can fetch deps without invalidating on source changes.
COPY Cargo.toml Cargo.lock* ./

# Pre-fetch dependencies (cached unless Cargo.toml/Cargo.lock change).
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo fetch --locked 2>/dev/null || cargo fetch

# --- Source layer ---
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY style/ ./style/
COPY public/ ./public/
COPY scripts/verify-wasm-bundle.sh ./scripts/verify-wasm-bundle.sh

# Full Leptos build — fail the image build if SSR, WASM, or JS artifacts are invalid.
# Cache mounts persist across builds on the same Railway builder instance.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo leptos build --release 2>&1 | tee /tmp/leptos-build.log && \
    cp target/release/onchainai /tmp/onchainai-bin && \
    cp -a target/site/pkg /tmp/pkg && \
    cp -a style /tmp/style && \
    cp -a public /tmp/public && \
    cp -a migrations /tmp/migrations && \
    cp Cargo.toml /tmp/Cargo.toml

RUN ln -sf onchainai.wasm /tmp/pkg/onchainai_bg.wasm
RUN bash scripts/verify-wasm-bundle.sh /tmp/pkg

RUN test -s /tmp/onchainai-bin \
    && test -s /tmp/pkg/onchainai.js \
    && test -s /tmp/pkg/onchainai.wasm \
    && test -e /tmp/pkg/onchainai_bg.wasm \
    && test -s /tmp/style/output.css \
    && test -s /tmp/public/chains/bitcoin.svg

# --- runtime stage ---
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /tmp/onchainai-bin /app/onchainai
COPY --from=builder /tmp/pkg /app/target/site/pkg
COPY --from=builder /tmp/Cargo.toml /app/Cargo.toml
COPY --from=builder /tmp/migrations /app/migrations
COPY --from=builder /tmp/style /app/style
COPY --from=builder /tmp/public /app/public

ENV PORT=3000
ENV RUST_LOG=info
# Leptos SSR + deep view trees need a larger tokio worker stack on Linux containers.
ENV RUST_MIN_STACK=8388608
ENV SKIP_CRAWLER=1
ENV LEPTOS_HYDRATION=1
EXPOSE 3000

# Railway + Docker pick this up to route traffic as soon as the app is ready,
# eliminating the edge-proxy "Application not found" gap after deploy.
HEALTHCHECK --interval=10s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -sf http://127.0.0.1:${PORT:-3000}/ || exit 1

CMD ["/app/onchainai"]
