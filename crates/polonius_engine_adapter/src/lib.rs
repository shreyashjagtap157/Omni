//! Compatibility boundary for the historical Polonius ownership oracle.
//!
//! Omni v0.1.4 does not qualify ownership-sensitive MIR. The former adapter
//! implementation is preserved under `docs/archive/unqualified-backends/` for
//! future v0.2 ownership work. This live crate intentionally fails closed so
//! enabling an old feature cannot create the appearance of a sound borrow check.

pub const OWNERSHIP_ORACLE_QUALIFIED: bool = false;

/// Reject ownership-oracle execution in the v0.1.4 baseline.
pub fn check_facts(_facts: &str) -> Result<(), String> {
    Err(
        "Polonius ownership-oracle execution is not qualified in Omni v0.1.4; ownership-sensitive MIR is reserved for the v0.2 ownership milestone"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oracle_fails_closed() {
        let error = check_facts("function main").expect_err("oracle must be unavailable");
        assert!(error.contains("not qualified"));
    }
}
