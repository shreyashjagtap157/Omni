# Changelog

All notable changes to the Omni project are documented in this file in reverse chronological order (newest on top).
The project adheres to four-part versioning: `stable.major.minor.patch`.
Releases and updates are categorized strictly under **Stable Releases**, **Major Updates**, and **Minor Updates** (with patches nested inside).

---

## Major Updates

### v0.2.0 — Ownership, Borrowing & Safe References
*Milestone Release: 2026-09-04 18:23:49 +05:30*

Qualified the complete v0.2.0 execution surface on owned x86-64 Linux ELF64 AOT:
- **Polonius Borrow Checker**: Integrated directly into the compiler driver pipeline (`crate::polonius::check_mir(&mir)`).
- **Linear Type Safety & CFG Analysis**: CFG linear resource movement and consumption verification, linear drop checks along all branching paths.
- **Reference Semantics**: Safe shared (`&x`) and mutable (`&mut x`) borrowing, reborrowing, and dereference assignment (`*r = expr`) without escaping loans.
- **Aggregate Field Mutation**: Direct nominal struct field mutation (`p.x = expr`) for `let mut` and `let linear` bindings.
- **Place Operations**: Partial field moves and linear field reinitialization.
- **Fail-Closed Diagnostics**: Rejection of escaping borrows, storing references in aggregates, and unproven borrows with stable diagnostics.
- **Test Integrity**: All 619 workspace tests pass (0 failed, 0 ignored).

#### Patches in v0.2.0:
- **v0.2.0.3** — *2026-09-05 14:25:00 +05:30*
  - Updated specification documents (`docs/archive/historical-plans/Omni_Complete_Specification.md` and `spec/README.md`) with normative v3.4 rules:
    - Added Section 5.10 (Provenance, Subobject Identity & Memory Observation Barriers).
    - Added Section 12.9 (Source-Order Observability & Reproducibility Envelope).
    - Added Section 17.5 (FFI and ABI Evaluation Order).
    - Added Appendix C (Five-Stage Qualification & Freeze Gating: Defined → Formalized → Implemented → Qualified → Frozen).
    - Added normative reference linking to `Omni_Complete_Specification_v3.4.md` and `CURRENT_IMPLEMENTATION_MATRIX.md`.
- **v0.2.0.2** — *2026-09-05 10:37:20 +05:30*
  - Included archived polonius package manifests into synchronized version bump targets.
- **v0.2.0.1** — *2026-09-05 10:36:43 +05:30*
  - Synchronized documentation (`README.md`, `AGENTS.md`, `IMPLEMENTATION_STATUS.md`, `INSTALL.md`, `GIT_SYNC_STATUS.md`) to reflect qualified v0.2.0.0 status.
  - Reorganized project directory by moving loose scratch scripts into `scripts/archive/` and test/audit logs into `docs/audit_logs/`.
- **v0.2.0.0** — *2026-09-05 10:31:03 +05:30*
  - Implemented automated four-part versioning (`stable.major.minor.patch`):
    - Added Git pre-commit hook (`.githooks/pre-commit` -> `scripts/auto-version-hook.py`) to auto-increment patch numbers on code commits without manual updates.
    - Added milestone bump automation `scripts/bump-version.py`.
    - Added remote GitHub synchronization scripts `scripts/sync-github.ps1` and `scripts/sync-github.sh`.
  - Added type unification and generic substitution logic in compiler (`crates/omni-compiler/src/type_checker.rs`).

---

### v0.1.0 — Native Bootstrap Pipeline Baseline
*Milestone Release: 2026-08-20 12:00:00 +05:30*

Established the core owned native compiler pipeline:
- Lexer, AST parser, nominal type resolution, and MIR generation.
- Owned x86-64 Linux ELF64 AOT generation without VM, JIT, or external linker dependencies.

---

## Minor Updates

### v0.1.4 — String, Byte, Value ABI & Collections Foundation
*Milestone Release: 2026-08-27 10:37:29 +05:30*

- Cross-function value ABI on owned x86-64 Linux AOT.
- Typed MIR/LIR function signatures and ABI value classes.
- Bounded indirect aggregate arguments and caller-owned aggregate return slots.
- Immutable `String` descriptors `{data, len}` with UTF-8 byte length, pass/return, and runtime print.
- Primitive `byte` values and binary `Bytes` descriptors distinct from UTF-8 strings.
- Binary-safe `b'X'`/`b"..."` literals, arbitrary byte transport, and bounds-checked `Bytes[index]`.
- Bootstrap `CellAllocator` contract and checked scalar-cell `OmniCellVector` with failure-atomic growth.
- Hardened MIR constant propagation: treated calls, spawns, indirect writes, and subobject writes as conservative observation barriers.

#### Patches in v0.1.4:
- **v0.1.4.1** — *2026-08-29 16:30:00 +05:30*
  - Formalized v3.4 five-stage traceability model (`Defined -> Formalized -> Implemented -> Qualified -> Frozen`) in release qualification manifests.
  - Closed 71 lineage audit findings across all compiler crates.

---

### v0.1.3 — Native Data Layout I
*Milestone Release: 2026-08-15 14:20:00 +05:30*

- Qualified first canonical local aggregate-layout wedge on owned x86-64 Linux AOT.
- Added nominal structs with declaration-order scalar-cell layout and checked field reads.
- Added structural tuples, homogeneous fixed arrays, runtime bounds-checked dynamic indexing, and non-escaping local slice views.
- Added nominal tagged enums with fieldless/scalar payload constructors and exhaustive variant matching.
- Added pre-emission alignment/frame validation and dedicated native bounds-fault path (exit 102).
- Fixed aggregate MIR liveness to protect initializer operands from elimination.
- Fuzzed lexer/parser for 60 cumulative seconds across 19,438 generated cases with zero failures.

---

### v0.1.2 — Scalar Control-Flow Hardening
*Milestone Release: 2026-08-10 11:00:00 +05:30*

- Implemented MIR `break`/`continue` for `loop` and `while`, with proper cleanup of locals in exited scopes.
- Added static errors 4014/4015 for loop control outside a loop.
- Added backend-independent MIR duplicate/dangling-label validation (7003).
- Added compiler/native tests for loop control and MIR validation.
- Added 23-case native scalar conformance corpus and runner.
- Added project-directory entry resolution for `check`, `run`, and `build`.

#### Patches in v0.1.2:
- **v0.1.2-r2** — *2026-08-12 18:00:00 +05:30*
  - Binary qualification of claimed v0.1.2 surface with Rust 1.97.1 on x86-64 Linux.
  - Resolved return inference, module declarations, and identifier control-condition ambiguity.
- **v0.1.2-r1** — *2026-08-11 09:30:00 +05:30*
  - Historical baseline remediation reconciling early 0.0.1–0.1.2 milestone contracts with Edition-1 target.
  - Strict formatter mode (`omni fmt --strict`) composable with `--check`.

---

### v0.1.1 — Native Core Baseline
*Milestone Release: 2026-08-01 10:00:00 +05:30*

- Owned x86-64 Linux ELF64 AOT backend with zero runtime dependencies.
- Made native AOT canonical `run`/`build` behavior.
- Added MIR parameter metadata and LIR/native scalar function calling.
- Hardened checked arithmetic in optimizer and development paths.
- Changed incomplete language features to fail closed with stable diagnostics.

---

## Stable Releases

*(Upcoming milestone per `docs/VERSIONING_AND_BOOTSTRAP_PLAN.md`: v1.0.0.0 — Complete Stable Omni Core)*
