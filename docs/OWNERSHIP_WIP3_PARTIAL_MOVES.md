# Omni 0.2.0.0 Ownership WIP3 — partial moves and lexical ownership

Status: WIP checkpoint, not a release qualification.

This checkpoint extends the 0.2.0.0 ownership work with source and MIR partial-place semantics.

Implemented:
- linear aggregate roots can be Available, PartiallyMoved, Moved, or MaybeMoved;
- moving one declared struct field preserves disjoint fields;
- whole-root use after a partial move is rejected;
- moving the same field twice is rejected;
- consuming every declared source field completes whole-value consumption;
- identical projected moves across branches merge exactly;
- mismatched projected moves across branches become conditional/MaybeMoved;
- branch/block-local linear values must be consumed before lexical scope exit;
- consumed branch locals are removed before the outer ownership join.

Validation on Rust/Cargo 1.97.1 with the offline locked dependency set:
- cargo fmt --all -- --check: PASS
- cargo test --locked --offline: PASS
- cargo clippy --locked --all-targets --offline -- -D warnings: PASS
- linear_type_checks: 21/21 PASS
- mir_field_projection: 6/6 PASS
- inherited v0.3.0 trait deferrals: 5 intentionally ignored

Still WIP:
- production safe references/regions are not present in this recovered WIP2 lineage;
- reinitialization after partial move;
- drop flags/destruction for partially moved aggregates;
- ownership-qualified native aggregate mutation;
- full 0.2.0.0 installed-compiler/native/fuzz qualification.
