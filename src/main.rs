//! OnchainAI server binary — Leptos SSR + Axum.

const TOKIO_WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(TOKIO_WORKER_STACK_SIZE)
        .build()
        .expect("tokio runtime")
        .block_on(onchainai::run_server())
        .expect("server failed");
}
