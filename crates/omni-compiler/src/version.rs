//! Project-wide user-facing Omni release versioning.
//!
//! Cargo itself only accepts SemVer-compatible package versions, so Cargo crate
//! manifests retain the three-component compatibility base. Omni release
//! artifacts, CLI identity, audits, and documentation use the project-wide
//! four-part scheme requested for all projects:
//!
//! `stableRelease.majorRelease.minorRelease.patch`

pub const PROJECT_VERSION: &str = "0.2.0.1";
pub const CARGO_SEMVER_BASE: &str = env!("CARGO_PKG_VERSION");
pub const VERSION_SCHEME: &str = "stable.major.minor.patch";

pub fn version_banner() -> String {
    format!("omni {}", PROJECT_VERSION)
}
