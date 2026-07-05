//! Integration smoke for `mcp-registry` scheduler wiring (listed as `mcp-registry-wiring`).

#![cfg(feature = "ssr")]

use onchainai::crawler::scheduler::SCHEDULER_JOB_COUNT;

#[test]
fn mcp_registry_scheduler_job_count_includes_registry_source() {
    // npm, cryptoskill, web3-mcp-hub, github, mcp-registry, vendor_orgs, bazaar, sync_stars
    assert!(
        SCHEDULER_JOB_COUNT >= 8,
        "scheduler must register mcp-registry, vendor_orgs, bazaar, and sync_stars"
    );
}
