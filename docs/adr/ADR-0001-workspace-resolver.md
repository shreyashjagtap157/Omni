# ADR-0001: Use Cargo workspace resolver = "2"

Status: Accepted

Date: 2026-04-15

Context
-------
The Omni project is a multi-crate Cargo workspace. The newer Cargo workspace resolver (v2) provides more deterministic dependency resolution and better handling of certain workspace edge cases.

Decision
--------
Set `resolver = "2"` in the root `Cargo.toml` under `[workspace]` to opt in to the v2 resolver for improved dependency resolution and future compatibility.

Consequences
------------
- Builds will use the v2 workspace resolver semantics across the workspace.
- This is compatible with current Rust toolchains that support the v2 resolver.

Notes
-----
This ADR also documents the small foundation additions (CONTRIBUTING, CODE_OF_CONDUCT) added on 2026-04-15 to improve developer onboarding and community expectations.
