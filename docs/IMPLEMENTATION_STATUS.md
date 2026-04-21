# Omni Implementation Status

Generated: 2026-04-21

This document is the current audit baseline after a full re-check against:

- `docs/plan.md` (roadmap steps 1-13)
- `docs/Omni_Complete_Specification.md` (phases 0-13)
- Actual code and test execution in this workspace

## Audit Summary

- Full workspace tests: green (`cargo test --workspace`)
- Advanced feature regression suite: green (`cargo test -p omni-compiler --test advanced_features`)
- LSP feature matrix: green in both manual and Salsa paths
- Polonius feature path: green (`--features use_polonius`)
- One concrete gap found and fixed in this pass: LSP completion support was missing from server capabilities and request handling; implemented end-to-end with regression coverage
- Follow-up fix in this pass: scoped async tasks now clean themselves up on scope exit, so the structured-concurrency surface no longer leaves pollable tasks behind after `finish()` or drop
- Follow-up work in this pass also completed the remaining advanced-feature surface and expanded the feature-gated LLVM lowering path beyond straight-line arithmetic
- Feature-gated real LLVM compile check reaches `llvm-sys`, but this workspace does not have a system LLVM 14 install or `LLVM_SYS_140_PREFIX` configured, so the inkwell-backed path remains environment-dependent here
- Added Stage0 declaration export infrastructure in `crates/omni-compiler/src/type_export.rs` with JSON and C-header output, plus ABI comparison helpers in `crates/omni-compiler/src/abi_check.rs`
- Added a Python `ctypes` binding scaffold on top of the same export data, so the Phase 11 interop surface now has a shared export layer for C and Python
- Added a minimal `codegen-wasm` backend crate that emits parseable WebAssembly bytes for the arithmetic subset used by `lir::example_module()` and validates the output with `wasmparser`
- Added `emit-wasm` to `crates/omni-stage0/src/main.rs` so the Stage0 CLI can write `.wasm` files for simple arithmetic source programs via the new wasm backend
- Exhaustive spec comparison still leaves major Phase 11 deliverables unimplemented in this workspace: production WebAssembly runtime/browser integration and full MLIR / GPU integration; the C/Python export layer is scaffolded but not a full production bindgen pipeline
- Step 11 is best read as three distinct sub-areas rather than one monolith: validated dev-path optimizations, a feature-gated LLVM release path, and a separate set of scaffolded interop/backend surfaces (WASM, MLIR, C/Python export)
- Phases 12 and 13 remain unstarted beyond infrastructure and documentation scaffolding

## Roadmap Step Status (`docs/plan.md`)

- Step 1 (project scaffolding): complete
- Step 2 (lexer / layout): complete
- Step 3 (parser / recovery / UI tests): complete
- Step 4 (resolver / type checker / effects): complete
- Step 5 (MIR / drop / borrow checks): complete (Stage0 path, adapter + mock solver + feature-gated real path)
- Step 6 (LIR/codegen + runtime fast path): complete for Stage0 bootstrap (Cranelift + fallback path)
- Step 7 (stdlib preservation strategy): complete for bootstrap surface (preserved originals retained)
- Step 8 (tests/fuzzing/diagnostics): complete for current Stage0 scope
- Step 9 (LSP): complete for current acceptance (go-to-def, hover, inlay hints, completion, workspace symbol indexing, rename support)
- Step 10 (advanced type/effect features): complete for the currently implemented surface (traits/comptime/async scope and generator helpers, macro expansion, effect composition, tuple/index support); variadic generics and specialization are still unimplemented
- Step 11 (optimizations/backends): partial
  - 11a: validated dev-path optimizations and Cranelift-backed execution
  - 11b: feature-gated real LLVM backend lowers control flow and calls, but still needs verified `with_inkwell` toolchain coverage
  - 11c: MLIR, WebAssembly runtime/browser integration, and full C/Python bindgen remain scaffolded or subset-only
- Step 12 (self-hosting migration): not complete
- Step 13 (platform/release maturity): not complete

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
- Phase 11: not complete
- Phase 12: not complete
- Phase 13: not complete

## What Was Implemented In This Audit Pass

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

## Verification Commands (this pass)

- `cargo test -p omni-compiler --test advanced_features`
- `cargo test -p omni-compiler --test lsp_incr_db`
- `cargo test -p omni-compiler --test type_export`
- `cargo test -p omni-compiler --features use_salsa_lsp --test lsp_salsa`
- `cargo test -p omni-compiler --features use_salsa_lsp,use_llvm`
- `cargo test -p codegen-llvm --lib`
- `cargo test -p codegen-wasm`
- `cargo test -p omni-compiler --features use_polonius`
- `cargo test --workspace`
- `cargo test --workspace --exclude codegen-llvm`
- `cargo run -p omni-stage0 -- export-types examples/function_call.omni python`
- `cargo run -p omni-stage0 -- emit-wasm tmp/wasm_emit_test.omni tmp/wasm_emit_test.wasm`

All commands above passed locally in this workspace.

## Remaining Work

1. Complete the real LLVM backend path with a verified `with_inkwell,real_llvm` environment and finish the remaining MLIR/tensor backend work.
2. Finish the missing Phase 11 surfaces that still require real backend work: production WebAssembly runtime/browser integration, full MLIR / GPU integration, and a production bindgen/ABI pipeline on top of the export layer.
3. Deliver self-hosting parity (Stage 1 == Stage 2) and platform/release maturity phases.

Generated: 2026-04-21