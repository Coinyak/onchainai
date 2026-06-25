# OnchainAI — multi-stage Docker build (SSR server + WASM hydration bundle).

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

RUN rustup target add wasm32-unknown-unknown

# Install cargo-leptos (portable across amd64/arm64 builders)
RUN cargo install cargo-leptos --locked --version 0.2.40

# Full Leptos build: SSR binary + WASM/JS client bundle
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