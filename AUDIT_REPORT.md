# Omni Local Audit Report

## 1. Environment & Git Status
- Host OS / Architecture: Windows AMD64
- Rust Toolchain (`rustc --version`): 1.97.1 (8bab26f4f 2026-07-14)
- Cargo version: 1.97.1 (c980f4866 2026-06-30)
- Current Git Branch & Commit: main @ 293086e feat: implement linear borrow checker for v0.2.0 ownership semantics
- Clean/Dirty Worktree Status: Dirty (1 modified file: crates/omni-compiler/src/type_checker.rs)

## 2. Versioning & Governance Consistency
- Root Cargo.toml / Workspace Version: 0.2.0.0 (project-version), 0.2.0 (cargo-semver-base)
- All crate manifest versions: 0.2.0.0 (omni-stage0, omni-compiler, omni-stdlib, codegen-native, codegen-cranelift, codegen-llvm, codegen-mlir, codegen-wasm, omni-release, lir, fuzz_harness)
- Version Constants:
  - `crates/omni-compiler/src/version.rs`: PROJECT_VERSION = "0.2.0.0", CARGO_SEMVER_BASE = "0.2.0"
  - `crates/codegen-native/src/lib.rs`: OMNI_PROJECT_VERSION = "0.2.0.0"
- Lineage Scripts Expected Version: 0.2.0.0
- `RELEASE_MANIFEST.json`: Not found as standalone file; version tracked in BINARY_QUALIFICATION.json
- `BINARY_QUALIFICATION.json`: version = "0.2.0.0", classification = "binary-qualified four-part-versioning patch for the cumulative native surface claimed through v0.1.4.1"
- Version Discrepancies: None detected; all sources consistent at 0.2.0.0

## 3. Test & Qualification Results
- `cargo fmt --all -- --check`: **FAIL** - formatting diff in type_checker.rs:789 (missing newline before comment block)
- `cargo clippy --workspace --locked --all-targets -- -D warnings`: **FAIL** - 12 compilation errors in omni-compiler type_checker.rs (missing `trait_system` argument in `check_expr` calls)
- `cargo test --workspace --locked`: **FAIL** - cannot compile due to same type_checker.rs errors
- `audit-baseline.py --worktree`: **PASS** - all 8 checkpoints passed, 0 errors
- `verify-source.py --worktree`: **PASS** - all 21 checkpoints passed, 0 errors
- Conformance Suites (positive):
  - native_scalar_v0_1_2: 23/23 cases passing
  - native_layout_v0_1_3: 10/10 cases passing
  - native_value_abi_v0_1_4: 10/10 cases passing
- Conformance Suites (negative/draft):
  - native_source_order_neg: 2 cases (arg_reorder_attempt, eval_canonicalization_rejected)
  - native_abi_neg: 2 cases (eval_order_attempt, abi_pack_reorder)
  - native_provenance_neg: 2 cases (provenance_loss, raw_exposure_fault)
  - native_freeze_neg: 2 cases (freeze_without_artifacts, qualified_fence_required)
  - native_continuation_neg: 2 cases (batch_isolation, wedge_sequencing)
- Ignored Tests Summary: 5 intentionally ignored v0.3.0.0 trait/type-system semantic enforcement tests (not v0.1.4.1 claimed surface)

## 4. Compiler Pipeline Analysis
- **Parser/Type Checker status**: **BROKEN** - `type_checker.rs` has 12 clippy errors and 1 fmt diff. Key issues:
  - `check_expr` function requires 6th argument `trait_system: &crate::traits::TraitSystem` but call sites at lines 1524, 1525, 1556, 1570, 1590, 1801, 2631, 2652, 2674 are missing it
  - Line 800 uses `trait_system` that is not in scope
  - Formatting inconsistency at line 789 (newline before comment)
- **Borrow checker status (Polonius / Region checks)**: **ACTIVE** - Linear borrow checker implemented for v0.2.0 ownership semantics (commit 293086e). Archived Polonius adapters exist but are not a soundness claim per v0.1.3 contract.
- **MIR Lowering & LIR state**: **Qualified subset** - typed params/returns, scalar/local aggregate/enum/String/Bytes operations covered. MIR control-flow hardening markers present (LoopContext, validate_control_flow, continue_target, break_target).
- **Native Codegen (x86_64 ELF64) coverage**: **Canonical v0.1.4.1.1 path** - codegen-native emits direct x86_64 Linux ELF64 without external linkers/VMs. Depends only on lir crate. Cranelift/LLVM/Wasm backends are explicitly unqualified/fail-closed.
- **Fail-closed boundaries status**: **ENFORCED** - Arithmetic faults exit 101, bounds faults exit 102, source order violations produce diagnostics 4014/4015, 'at most 6' integer params rejected, aggregate field mutation rejected until v0.2.0 ownership is qualified.

## 5. Identified Gaps & Priority Reconciliation Issues
- **BLOCKER**: `cargo clippy` and `cargo test` fail to compile the workspace due to 12 errors in `crates/omni-compiler/src/type_checker.rs`. The `check_expr` function signature was updated to require a `trait_system` parameter, but all 9 call sites were not updated. This is a **blocking compilation issue** that prevents any workspace test or clippy verification.
- **BLOCKER**: `cargo fmt --all -- --check` reports formatting diff in type_checker.rs line 789. The file needs formatting alignment.
- **Technical rationale**: The type_checker.rs changes appear to be part of the v0.2.0 linear borrow checker implementation (commit 293086e). The `trait_system` parameter was added to `check_expr` but the migration of call sites was incomplete. This must be completed before the workspace can compile.
- **Proposed immediate next batch**: **Batch 0 Version Sync Fix** - Fix the `check_expr` call sites to pass `trait_system` argument, and run `cargo fmt` to align formatting. Rationale: Without this, no workspace tests or clippy checks can run, blocking all downstream qualification verification.
- **Secondary gaps** (after Batch 0):
  - Expand native conformance corpus to cover v3.4 source-order observability regression tests
  - Add per-pass provenance-preservation audit in the implementation matrix
  - Complete negative conformance test execution (currently Windows-hosted, requires Linux/WSL)
  - Document freeze/qualification state as a separate v3.4 gate

## Collaboration Protocol
As per the audit guidelines:
1. No speculative code changes or hack workarounds will be applied without Lead Architect approval.
2. The identified blocker in type_checker.rs requires technical debate before modifying - specifically whether the `trait_system` parameter should be threaded through from the parent check function context, or if the bounded-check code at lines 800-802 should be restructured to avoid the parameter dependency.
3. The fmt diff and clippy errors are likely related - both stem from the incomplete migration when `trait_system` was added to `check_expr`'s signature.
4. Before any code modifications, the action plan should be: (a) identify the `trait_system` value available at each call site, (b) either thread it through or restructure the bounded-check logic, (c) run `cargo fmt --all` to resolve formatting, (d) verify `cargo clippy` and `cargo test` pass, then (e) re-run audit scripts.