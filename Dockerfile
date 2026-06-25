# OnchainAI — multi-stage Docker build (SSR binary + WASM hydration bundle).
# Leptos 0.8 requires rustc 1.88+ and cargo-leptos for /pkg/*.js + *.wasm.

FROM rust:1.88-slim AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev curl perl \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add wasm32-unknown-unknown \
    && curl -fsSL https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-gnu.tgz \
        | tar -xz -C /usr/local/cargo/bin \
    && cargo binstall cargo-leptos -y

COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY style/ ./style/

# SSR server binary + WASM/JS client bundle → target/site/pkg/
RUN cargo leptos build --release

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