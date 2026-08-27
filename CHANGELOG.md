## v3.4 Spec Reconciliation Notes

- Made MIR constant propagation treat calls, spawns, indirect writes, and aggregate
  subobject writes as conservative observation/memory barriers, preventing stale facts
  from changing post-effect source meaning.
- Added optimizer regressions for reference-reachable mutation, indirect/subobject
  writes, concurrent spawn boundaries, and preservation of observable instruction order.
- Added the converged v3.4 specification archive and adjudication/implementation
  companion docs under `docs/archive/historical-plans/`.
- Updated the live specification redirect to point at the v3.4 archive as the current
  historical authority.
- Reframed the implementation matrix as a v3.4 reconciliation document so the current
  bootstrap status is tracked against the converged rules rather than only the older
  milestone language.
- Added an explicit v3.4 gap summary covering source-order observability, reproducible
  artifact rules, provenance traceability, ABI/FFI order, and freeze-gating gaps.

## 0.1.3 — Native Data Layout I

- Qualified the first canonical local aggregate-layout wedge on owned x86-64 Linux AOT.
- Added nominal structs with declaration-order scalar-cell layout and checked field reads.
- Added structural tuples, homogeneous fixed arrays, runtime bounds-checked dynamic indexing, and non-escaping local slice views.
- Added nominal tagged enums with fieldless/scalar payload constructors and exhaustive variant matching.
- Added pre-emission alignment/frame validation and a dedicated native bounds-fault path (exit 102).
- Fixed aggregate MIR liveness so optimization cannot delete aggregate initializer operands.
- Fixed aggregate type registration to be two-pass, nominal, and fail closed on unknown field/payload types while supporting forward nominal references.
- Kept aggregate argument/return/string value ABI fail closed for v0.1.4.1.1 and ownership-sensitive mutation/nontrivial destruction fail closed for v0.2.0.
- Workspace qualification passes with 548 registered tests; only five v0.3.0 trait-semantic tests remain intentionally ignored.
- Historical compatibility passes 5/5, native scalar conformance 23/23, and native-layout conformance 10/10.
- Fuzzed the final lexer/parser for 60 cumulative seconds across 19,438 generated cases and four independent seeds with zero crashes/signals/timeouts.

## 0.1.2-r2 — Binary Qualification

- Binary-qualified the complete feature surface claimed through v0.1.2 with Rust 1.97.1 on x86-64 Linux.
- All five requested Cargo gates pass; whole-workspace Clippy/tests also pass.
- Historical compatibility is 5/5 and native scalar conformance is 23/23.
- Fuzzed the final lexer/parser for 60 cumulative seconds across 22,080 generated cases with zero crashes/signals/timeouts.
- Fixed return inference, cross-module signature import, assignment parsing, semicolon round-trips, historical module declarations, brace-struct closure, identifier control-condition ambiguity, workspace Wasm/LLVM integrity, and native launch robustness.
- Expanded the lineage remediation register to 71 closed findings and separated local worktree audits from strict lean-source-package checks.
- cargo-fuzz remains an optional deeper oracle; the required duration-based fuzz gate is self-contained/offline.

## 0.1.2-r1 — Historical Baseline Remediation

- Reconciles the original cumulative 0.0.1–0.1.2 milestone contracts with the Edition-1 target.
- Preserves error-set visibility through AST/parser/formatter.
- Exposes strict formatter mode through `omni fmt --strict`, composable with `--check`.
- Adds historical milestone closure regression coverage and a release closure manifest.
- Tracks 49 closed lineage findings and corrects conformance-case metadata drift.
- Remains source-closed but requires pinned-Rust host build/test/lint/native/fuzz qualification before binary promotion.

# Changelog

## 0.1.2 — Scalar Control-Flow Hardening

- Implemented MIR `break`/`continue` for `loop` and `while`, including cleanup of
  locals in scopes exited by the control transfer.
- Added static errors 4014/4015 for loop control outside a loop.
- Added backend-independent MIR duplicate/dangling-label validation (7003).
- Added compiler/native tests for loop control and MIR validation.
- Added 10-case native scalar conformance corpus and runner.
- Added project-directory entry resolution for `check`, `run` and `build`.
- Made `omni new` emit an Edition-2026 brace/semicolon native-smoke project.
- Made project builds write native output to `target/omni/<package>` by default and
  generate the current `omni.lock`.
- Retained the minimal default backend closure: Cranelift/Wasm/LLVM remain opt-in.

## 0.1.1 — Native Core Baseline

- Added owned x86-64 Linux ELF64 AOT backend with no VM/JIT/LLVM/assembler/linker
  dependency in the emission path.
- Made native AOT the canonical `run`/`build` behavior; retained Cranelift as explicit
  development `run-jit` path.
- Restored missing generic-codegen gate and removed stale missing Cranelift AOT module.
- Added MIR parameter metadata and LIR/native scalar function calling.
- Removed hard-coded function-name specialization.
- Fixed unary minus lowering.
- Hardened checked arithmetic in optimizer/development paths.
- Preserved calls and faulting operations across optimization.
- Tightened constant inlining to side-effect-free constant-return bodies.
- Fixed `while` CFG labels and unresolved jump patch behavior.
- Changed incomplete language features to fail closed instead of emitting invented
  scalar semantics.
- Added installation/source-verification scripts, current status audit, native policy,
  and versioned bootstrap roadmap.
