# OnchainAI — multi-stage Docker build (SSR server + WASM hydration bundle).
# Cache-bust: 2026-06-25T16:30Z

FROM rust:1.90-slim AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential pkg-config libssl-dev curl perl \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY style/ ./style/

RUN rustup target add wasm32-unknown-unknown

# Install cargo-leptos (portable across amd64/arm64 builders)
ARG CARGO_LEPTOS_VERSION=0.3.6
RUN cargo install cargo-leptos --version "${CARGO_LEPTOS_VERSION}" --locked

# Full Leptos build — fail the image build if SSR, WASM, or JS artifacts are invalid.
RUN cargo leptos build --release 2>&1 | tee /tmp/leptos-build.log

RUN test -s /app/target/release/onchainai \
    && test -s /app/target/site/pkg/onchainai.js \
    && test -s /app/target/site/pkg/onchainai.wasm \
    && test -s /app/style/output.css

# --- runtime stage ---
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/onchainai /app/onchainai
COPY --from=builder /app/target/site /app/target/site
COPY --from=builder /app/Cargo.toml /app/Cargo.toml
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/style /app/style

ENV PORT=3000
ENV RUST_LOG=info
# Leptos SSR + deep view trees need a larger tokio worker stack on Linux containers.
ENV RUST_MIN_STACK=8388608
ENV SKIP_CRAWLER=1
EXPOSE 3000

CMD ["/app/onchainai"]