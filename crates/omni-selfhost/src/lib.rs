pub mod bootstrap;

pub use bootstrap::{build_stage1, build_stage2, compare_stages, SelfHostError};

pub const VERSION: &str = "0.1.0-stage0";

pub fn version() -> &'static str {
    VERSION
}