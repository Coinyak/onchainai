//! OnchainAI server binary — Leptos SSR + Axum.

fn main() {
    tokio::runtime::Runtime::new()
        .expect("tokio runtime")
        .block_on(onchainai::run_server())
        .expect("server failed");
}