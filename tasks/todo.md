# Omni Implementation Roadmap

## Completed Milestones

### Immediate — Restore Green Gates & Workspace Integrity (Completed)
- [x] Revert broken `trait_system` WIP in `driver.rs` / `type_checker.rs`
- [x] Remove dead `trait_bounds` scaffolding from `InferCtx` (clippy)
- [x] Clean and consolidate trait bound tests in `type_inference_ui.rs` (all 6 tests passing)
- [x] Delete incomplete `fix_driver.py` / `fix_trait.py` helper scripts
- [x] Re-run full qualification tests (`verify-source.py`, `audit-baseline.py`, cargo fmt/clippy/test: 619 passing)
- [x] Synchronize `release/BINARY_QUALIFICATION.json`, `SOURCE_QUALIFICATION.json`, and `RELEASE_MANIFEST.json`

### v0.2.0 — Ownership & Borrowing Wedge (Completed)
- [x] Design production borrow checker (integrated `crate::polonius::check_mir(&mir)`)
- [x] Wire linear dataflow checks into compiler driver before LIR lowering
- [x] Enable aggregate field mutation (`p.x = expr`) with place-based and mutability validation
- [x] Build ownership conformance and integration test corpus (`mir_borrow_checks`, `local_reference_v0_2_0`, `ownership_reinit_native_v0_2_0`)
- [x] Implement four-part automated versioning (`stable.major.minor.patch`), auto-increment git hook, and remote sync tooling
- [x] Reorganize loose repository files into `scripts/archive/` and `docs/audit_logs/`

---

## Remaining Implementation Roadmap

### v3.4 Traceability & Conformance Batch
The following points from the v3.4 reconciliation gap analysis (`docs/archive/historical-plans/Omni_v3.4_Gap_List.md`) remain to be executed:

- [x] **Batch 0 — Traceability Links**:
  - [x] Add explicit link to `docs/archive/historical-plans/Omni_v3.4_Gap_List.md` from `CURRENT_IMPLEMENTATION_MATRIX.md` and `ROADMAP.md`
  - [x] Expand the single v3.4 traceability table in `CURRENT_IMPLEMENTATION_MATRIX.md` (Defined → Formalized → Implemented → Qualified → Frozen)
- [ ] **Batch 1 — Source-Order Observability**:
  - [ ] Promote `conformance/native_source_order_neg/` from draft to active qualification runner
  - [ ] Expand native corpus for nested call argument-order regressions
  - [ ] Map diagnostics to strict v3.4 fail-closed diagnostic codes
- [ ] **Batch 2 — Provenance Audit**:
  - [x] Document per-pass provenance preservation (`AST -> MIR -> LIR -> native`) in `CURRENT_IMPLEMENTATION_MATRIX.md`
  - [ ] Promote `conformance/native_provenance_neg/` to active gate
- [ ] **Batch 3 — ABI/FFI Evaluation Order**:
  - [ ] Promote `conformance/native_abi_neg/` to active qualification gate
  - [ ] Add negative ABI argument and packing reordering test cases
- [ ] **Batch 4 — Reproducibility Envelope**:
  - [ ] Prove metadata canonicalization cannot alter runtime semantics
  - [ ] Promote `conformance/native_freeze_neg/` and `native_continuation_neg/` to gate

### v0.3.0 — Generics, Traits & Monomorphization (Next Major Feature Wedge)
- [ ] **Full Trait Resolution Pipeline**:
  - [ ] Expand `TraitSystem` to resolve associated types and default method implementations
  - [ ] Thread full trait coherence checks (orphan rules, duplicate impls, overlapping blanket impls)
- [ ] **Native Monomorphization**:
  - [ ] Monomorphize generic struct and enum layouts at native code generation
  - [ ] Lower generic function instances to specialized symbol mangling in `codegen-native`
  - [ ] Add positive and negative native conformance suites for generic types and trait calls

---

## Review

*All 619 workspace tests and all lineage audit scripts are green. Milestone v0.2.0.0 is verified and active.*
