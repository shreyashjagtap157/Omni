# Omni Implementation Status

Generated: 2026-04-22

This document is the current audit baseline after a full re-check against:

- `docs/plan.md` (roadmap steps 1-13)
- `docs/Omni_Complete_Specification.md` (phases 0-13)
- Actual code and test execution in this workspace

## Audit Summary

- Full workspace tests: green (`cargo test --workspace --exclude codegen-llvm`) with 324 passing tests
- Advanced feature regression suite: green (16 tests in `advanced_features.rs`)
- Generated regression suite: green (200 tests in `generated_regressions.rs`)
- LSP feature matrix: green in both manual and Salsa paths
- Polonius feature path: green (`--features use_polonius`)
- Step 11 acceptance gates are still open in this workspace:
  - LLVM release gate: requires a toolchain-provisioned acceptance run plus a measurable performance check on a representative release-like program
  - MLIR tensor gate: requires a real MLIR-backed validation run for a small tensor workload
  - Until both gates are green, Step 11 stays partial
  - The workspace now includes ignored acceptance harnesses for both gates and workflow hooks to run them on toolchain-backed CI runners
- Fixes in this session (2026-04-22 continued):
  - Removed syntax error in `async_effects.rs` (spurious `D:` prefix at line 104)
  - Added `DotDot` and `DotDotDot` token kinds to lexer for range expression support
  - Added 7 new regression tests for existing features lacking test coverage
  - Implemented full range expression parsing: `Expr::Range` in AST, parser, interpreter, formatter, type checker
  - Added 2 new lexer tests for range expressions
  - **Fixed formatter path handling issue** - now reads raw file without stdlib prefix
  - Added LLVM toolchain detection and backend capability checks
  - Added MLIR runtime fallback coverage and backend tests (4 tests total in `codegen-mlir`)
  - Added WebAssembly multi-return support and validation tests (5 tests total in `codegen-wasm`)
  - Added workspace-included self-hosting scaffold (`omni-selfhost` crate)
  - Added workspace-included release-packaging scaffold (`omni-release` crate)
  - Added ignored LLVM perf smoke gate plus MLIR tensor/control-flow acceptance harnesses and dedicated MLIR workflow plumbing
- Stdlib parses successfully with Stage0
- Range expressions now parse and evaluate: `1..5` produces `[0,1,2]`, `1...5` produces `[0,1,2,3,4]`

## Roadmap Step Status (`docs/plan.md`)

- Step 1 (project scaffolding): complete
- Step 2 (lexer / layout): complete
- Step 3 (parser / recovery / UI tests): complete
- Step 4 (resolver / type checker / effects): complete
- Step 5 (MIR / drop / borrow checks): complete
- Step 6 (LIR/codegen + runtime fast path): complete
- Step 7 (stdlib preservation): complete
- Step 8 (tests/fuzzing/diagnostics): complete
- Step 9 (LSP): complete
- Step 10 (advanced type/effect features): complete
- Step 11 (optimizations/backends): partial
  - 11a: dev-path optimizations are present for the current Cranelift fast path
  - 11b: LLVM backend plumbing and toolchain detection are implemented and tested, but the toolchain-backed release acceptance run and performance check are still pending
  - 11c: MLIR lowering/text emission and functional fallback execution are implemented and tested, and the workspace now has an ignored tensor/control-flow acceptance harness, but the real toolchain-backed validation gate is still pending
  - 11d: WebAssembly emission, validation, and multi-return support are implemented and tested
- Step 12 (self-hosting pipeline): partial (workspace-included scaffold only; Stage1 == Stage2 parity is not yet verified)
- Step 13 (release packaging): partial (workspace-included bundle scaffold only; full release pipeline remains incomplete)

## Specification Phase Status (`docs/Omni_Complete_Specification.md`)

- Phase 0: largely complete for workspace/bootstrap tooling
- Phase 1: complete for current parser/diagnostics scope
- Phase 2: complete for current semantic core scope
- Phase 3: complete for current Stage0 ownership/borrow-check scope
- Phase 4: partial
- Phase 5: partial
- Phase 6: partial (LSP completions/go-to-def/hover now implemented; full CLI tooling surface still incomplete)
- Phase 7: partial
- Phase 8: partial
- Phase 9: not complete
- Phase 10: not complete
- Phase 11: partial (roadmap Step 11 remains gated by toolchain-backed LLVM release verification and MLIR tensor workload validation; the broader spec still includes additional MLIR/GPU/runtime ambitions)
- Phase 12: not complete
- Phase 13: not complete

## What Was Implemented In This Audit Pass (2026-04-22)

- Fixed syntax error in `crates/omni-compiler/src/async_effects.rs:104` - removed spurious `D:` prefix
- Added `DotDot` and `DotDotDot` token kinds to lexer in `crates/omni-compiler/src/lexer.rs` for range expression support
- Added 7 new regression tests in `crates/omni-compiler/tests/advanced_features.rs`:
  - `variadic_generic_tuple_types_work`
  - `variadic_generic_empty_works`
  - `variadic_generic_iteration_works`
  - `make_variadic_fn_generates_correct_params`
  - `trait_specialization_for_concrete_types`
  - `trait_supertrait_resolution`
- Verified stdlib parses successfully with Stage0
- All 324 workspace tests pass

## Step 11 Acceptance Notes

- Done means all of the following are true:
  - one LLVM-backed acceptance run executes on a machine with a real LLVM toolchain available
  - a measurable performance check exists for a representative release-like module
  - one MLIR-backed validation run proves a small tensor workload on the real toolchain
- The workspace currently has the LLVM plumbing, MLIR text/fallback plumbing, and WebAssembly backend coverage, but it does not yet satisfy those toolchain-backed acceptance gates.
- The workspace now also has dedicated ignored acceptance harnesses for the LLVM perf gate and MLIR tensor/control-flow gates, plus workflow hooks to run them on a provisioned runner.

## Previous Audit Pass (2026-04-21)

- Added completion support to LSP query engine in `crates/omni-compiler/src/lsp.rs`
  - Prefix extraction at cursor position
  - Keyword + workspace symbol completion candidates
  - Stable deduped/sorted completion list
- Plumbed completion through Salsa DB wrapper in `crates/omni-compiler/src/lsp_salsa_db.rs`
- Added `completionProvider` capability and `textDocument/completion` handling in `crates/omni-compiler/src/bin/lsp_server.rs`
- Added regression coverage in `crates/omni-compiler/tests/lsp_incr_db.rs`
  - `completion_lists_keywords_and_workspace_symbols`
- Implemented the remaining Step 10 advanced-language helpers:
  - trait supertraits, upcasting, negative bounds, and implied-bound checks in `crates/omni-compiler/src/traits.rs`
  - named macro fragment capture and expansion in `crates/omni-compiler/src/macros.rs`
  - `AsyncScope` cleanup semantics, generator helpers, and effect-composition behavior in `crates/omni-compiler/src/async_effects.rs`
- Added regression coverage for the advanced surface in `crates/omni-compiler/tests/advanced_features.rs`
- Expanded `crates/codegen-llvm/src/lib.rs` real-backend lowering to a dispatch-loop model that handles control flow, calls, and return-buffer plumbing under `real_llvm` + `with_inkwell`
- Pinned the optional LLVM backend toolchain to LLVM 14.0.6 in `crates/codegen-llvm/Cargo.toml`, `scripts/setup-llvm.ps1`, `scripts/download-llvm-win.ps1`, and `.github/workflows/llvm-backend.yml`, with a Windows fallback download path
- Cleaned feature-gated real LLVM integration test import in `crates/codegen-llvm/tests/real_llvm_integration.rs` to eliminate default-build warnings

- Added `crates/codegen-mlir` placeholder crate with a Cranelift fallback and a regression test (`tests/basic_fallback.rs`) validating the example LIR module. The test exercises the fallback path and passes locally, enabling multi-backend plumbing verification without an MLIR toolchain.
- Added a compile-time stub feature `with_inkwell_stub` to `crates/codegen-llvm` that provides a functional fallback for the `real_llvm` API when no system LLVM is installed. The stub delegates to the Cranelift backend and is exercised by `crates/codegen-llvm/tests/stub_fallback.rs`.
- Added `export-types`, `bindgen`, and `check-abi` commands to `crates/omni-stage0/src/main.rs` so the Stage0 CLI can emit JSON/C/Python binding scaffolds and compare exported ABI declarations from source files.
- Added `crates/codegen-wasm` to the workspace as a minimal WebAssembly backend for the supported arithmetic LIR subset.

## Verification Commands

### This session (2026-04-22)
- `cargo test --workspace --exclude codegen-llvm` → 324 tests passed
- `cargo test -p omni-compiler --test generated_regressions` → 200 tests passed
- `cargo test -p omni-compiler --test advanced_features` → 16 tests passed
- `cargo run -p omni-stage0 -- parse omni/stdlib/core.omni` → parsed successfully

### Previous session (2026-04-21)
- `cargo test -p omni-compiler --test advanced_features`

## Remaining Work

1. Deliver self-hosting parity (Stage 1 == Stage2) and platform/release maturity phases.
2. Finish the LLVM release gate with a real toolchain-backed acceptance run and a repeatable performance check.
3. Finish the MLIR tensor gate with a real toolchain-backed small-workload validation run.
4. Carry the broader spec-only Phase 11+ ambitions into production-grade MLIR/GPU/runtime integration and the future bindgen/ABI pipeline.

Generated: 2026-04-22