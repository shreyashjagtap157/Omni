//! Release-publication boundary for the v0.1.4 bootstrap.
//!
//! Official packaging, signing, provenance, SBOM generation, and publication
//! are later roadmap gates. This binary exists so automation gets an explicit
//! failure instead of a deceptively successful partial archive.

fn main() {
    eprintln!(
        "omni-release: official release packaging is not qualified in Omni v0.1.4; \
         use the repository source ZIP plus SHA256SUMS for bootstrap evaluation"
    );
    std::process::exit(2);
}
