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
- [x] **Batch 1 — Source-Order Observability**:
  - [x] Promote `conformance/native_source_order_neg/` from draft to active qualification runner
  - [x] Expand native corpus for nested call argument-order regressions
  - [x] Map diagnostics to strict v3.4 fail-closed diagnostic codes
- [x] **Batch 2 — Provenance Audit**:
  - [x] Document per-pass provenance preservation (`AST -> MIR -> LIR -> native`) in `CURRENT_IMPLEMENTATION_MATRIX.md`
  - [x] Promote `conformance/native_provenance_neg/` to active gate
- [x] **Batch 3 — ABI/FFI Evaluation Order**:
  - [x] Promote `conformance/native_abi_neg/` to active qualification gate
  - [x] Add negative ABI argument and packing reordering test cases
- [x] **Batch 4 — Reproducibility Envelope**:
  - [x] Prove metadata canonicalization cannot alter runtime semantics
  - [x] Promote `conformance/native_freeze_neg/` and `native_continuation_neg/` to gate

### v0.3.0 — Generics, Traits & Monomorphization (Completed)
- [x] **Full Trait Resolution Pipeline**:
  - [x] Expand `TraitSystem` with duplicate implementation coherence checks
  - [x] Thread trait coherence error reporting through compiler pipeline
- [x] **Native Monomorphization**:
  - [x] Concrete AST monomorphization engine in `crates/omni-compiler/src/monomorphizer.rs`
  - [x] Specialize generic functions into concrete signatures (`_i64`) and rewrite calls
  - [x] Skip unspecialized generic functions during MIR generation to ensure pure concrete code generation
  - [x] Add positive and negative native conformance suites for Batches 1–4 and generic handling
- [x] **Version Advance**:
  - [x] Advance workspace to `v0.3.0.0` across manifests, JSON artifacts, and compiler version constants

---

## Review

*All workspace tests (100%), native conformance suites (Batches 1–4), and lineage baseline audits pass completely. Milestone v0.3.0.0 is verified and active.*
