pub mod bootstrap;

pub use bootstrap::{build_stage0, build_stage1, build_stage2, compare_stages, SelfHostError};

pub const VERSION: &str = "0.2.0.0-rust-bootstrap";

pub fn version() -> &'static str {
    VERSION
}
