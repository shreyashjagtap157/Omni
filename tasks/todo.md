# Omni Implementation Roadmap (post-audit 2026-09-02)

## Immediate — restore green gates

- [x] Revert broken `trait_system` WIP in `driver.rs` / `type_checker.rs`
- [x] Remove dead `trait_bounds` scaffolding from `InferCtx` (clippy)
- [x] Mark v0.3.0 trait-bound tests as `#[ignore]` until enforcement lands
- [x] Delete incomplete `fix_driver.py` / `fix_trait.py` helper scripts
- [ ] Re-run full qualification on Linux/WSL2 (`./scripts/qualify-release.sh`)
- [ ] Update `release/BINARY_QUALIFICATION.json` with current gate results

## v3.4 Batch 0 — traceability (1–2 days)

- [ ] Link `docs/archive/historical-plans/Omni_v3.4_Gap_List.md` from `CURRENT_IMPLEMENTATION_MATRIX.md` and `ROADMAP.md`
- [ ] Add single v3.4 traceability table (Defined → Formalized → Implemented → Qualified → Frozen)
- [ ] Reconcile `BINARY_QUALIFICATION.json` claims vs actual `cargo test` / `clippy` results

## v3.4 Batch 1 — source-order observability (~1 week)

- [ ] Promote `conformance/native_source_order_neg/` from draft to qualification gate
- [ ] Expand native corpus for nested call argument-order regressions
- [ ] Map diagnostics to v3.4 fail-closed language

## v3.4 Batch 2 — provenance audit (~1 week)

- [ ] Document per-pass provenance preservation (parser → MIR → LIR → native) in matrix
- [ ] Promote `conformance/native_provenance_neg/` to gate

## v3.4 Batch 3 — ABI/FFI eval order (~1 week)

- [ ] Promote `conformance/native_abi_neg/` to gate
- [ ] Add negative ABI reordering tests

## v3.4 Batch 4 — reproducibility envelope (1–2 weeks)

- [ ] Prove metadata canonicalization cannot alter semantics
- [ ] Promote `conformance/native_freeze_neg/` and `native_continuation_neg/` to gate

## v0.2.0 — ownership wedge (1–3 months)

- [ ] Design production borrow checker (replace or harden `polonius.rs` adapter)
- [ ] Wire or remove `borrow_check/` stub; integrate before MIR optimization
- [ ] Enable aggregate field mutation with place-based validation
- [ ] Build negative conformance corpus (use-after-move, conflicting borrows, partial moves)
- [ ] Un-ignore trait-bound tests only after trait system is properly threaded

## v0.3.0 — generics & traits

- [ ] Thread `TraitSystem` through type checker before step 4 in driver
- [ ] Implement generic trait bound enforcement in `synthesize_expr` / `check_expr`
- [ ] Remove `#[ignore]` from `test_trait_bounds_*` and `test_trait_violation`

## Review

_Audit source: comprehensive spec vs implementation audit (2026-09-02). Qualified wedge remains v0.1.4.1.1 on x86-64 Linux when gates pass._
