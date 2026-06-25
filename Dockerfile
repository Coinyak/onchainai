# OnchainAI — multi-stage Docker build (SSR server + optional WASM hydration bundle).

FROM rust:1.88-slim AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev curl perl \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY style/ ./style/

# SSR binary (required)
RUN cargo build --release

# WASM/JS client bundle (optional — app skips hydration when pkg/onchainai.js is absent)
RUN mkdir -p target/site/pkg \
    && (rustup target add wasm32-unknown-unknown \
        && curl -fsSL https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-gnu.tgz \
            | tar -xz -C /usr/local/cargo/bin \
        && cargo binstall cargo-leptos -y \
        && cargo leptos build --release) \
    || echo "WASM bundle build skipped; SSR-only mode"

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
EXPOSE 3000

CMD ["/app/onchainai"]