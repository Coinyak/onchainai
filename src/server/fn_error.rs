//! Shared error type for server business logic (replaces Leptos `ServerFnError`).

#[derive(Debug, Clone)]
pub struct FnError(String);

impl FnError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl std::fmt::Display for FnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for FnError {}
