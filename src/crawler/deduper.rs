//! Deduper — removes duplicates by `repo_url`, keeping the first entry.
//!
//! Tools with `None` repo_url are preserved.

/// Remove duplicate tools by `repo_url`, keeping the first occurrence.
///
/// Implemented in a later milestone; exposed here so the crawler module
/// compiles during foundation setup.
#[allow(dead_code)]
pub fn dedupe(_tools: Vec<String>) -> Vec<String> {
    Vec::new()
}
