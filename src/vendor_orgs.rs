//! Shared vendor org manifest — `scripts/vendor-orgs.json` (PR-2).
//!
//! Parsed once via [`vendor_orgs_manifest`] for PR-4 vendor-org crawler reuse.

use std::sync::OnceLock;

/// One curated first-party GitHub org entry.
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct VendorOrgEntry {
    pub github: String,
    pub team: String,
    pub crawl: bool,
    #[serde(default)]
    pub npm_scopes: Vec<String>,
}

/// Top-level vendor org manifest (`version` + `orgs`).
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct VendorOrgsManifest {
    pub version: u32,
    pub orgs: Vec<VendorOrgEntry>,
}

static VENDOR_ORGS: OnceLock<VendorOrgsManifest> = OnceLock::new();

/// Lazily parse the embedded `scripts/vendor-orgs.json` manifest.
pub fn vendor_orgs_manifest() -> &'static VendorOrgsManifest {
    VENDOR_ORGS.get_or_init(|| {
        let raw = include_str!("../scripts/vendor-orgs.json");
        serde_json::from_str(raw).expect("parse vendor-orgs.json")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_orgs_json_embed_parses_and_includes_circlefin() {
        let manifest = vendor_orgs_manifest();
        assert_eq!(manifest.version, 1);
        assert!(manifest.orgs.len() >= 45);

        let circlefin = manifest
            .orgs
            .iter()
            .find(|entry| entry.github == "circlefin")
            .expect("circlefin entry");
        assert_eq!(circlefin.team, "Circle");
        assert!(circlefin.crawl);
        assert!(circlefin
            .npm_scopes
            .iter()
            .any(|scope| scope == "@circle-fin"));
    }
}
