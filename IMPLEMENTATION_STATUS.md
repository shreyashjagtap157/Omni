# Omni implementation status

The authoritative detailed status is `docs/CURRENT_IMPLEMENTATION_MATRIX.md`.

Current milestone: **v0.2.0.0 — Ownership, Borrowing & Safe References**.

The cumulative v0.0.1 through v0.1.4.1 scalar, control-flow, local-layout, and value-ABI baseline
remains intact. v0.2.0.0 qualifies the production ownership and borrow-checking execution wedge:
Polonius MIR borrow verification, linear resource consumption checks along CFG paths, safe shared
and mutable reference borrowing (`&x`, `&mut x`), reborrowing, dereference assignment (`*r = expr`),
partial field moves, linear field reinitialization, and struct field mutation (`p.x = expr`) for
`let mut` and `let linear` bindings.

This milestone deliberately does **not** claim full generic/trait-native monomorphization (v0.3.0.0),
effects/capabilities (v0.4.0.0), structured concurrency/async (v0.5.0.0), or multiple canonical native
ISAs/PE/Mach-O backends (v0.7.0.0). Those paths remain fail closed.

Promotion requires source audits, workspace fmt/Clippy/tests/build (619+ tests passing), historical 5-case
compatibility, the 23-case scalar corpus, 10-case v0.1.3 layout corpus, 10-case v0.1.4.1 value-ABI corpus,
v0.2.0 ownership/borrow test suites, and deterministic fuzz gates on the pinned Rust 1.97.1 host.
