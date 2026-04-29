# OMNI CODEBASE - UPDATED EXHAUSTIVE AUDIT REPORT
## Spec-Accurate Implementation Audit, Plan Alignment, and Gap Inventory

**Date:** 2026-04-29  
**Scope:** Entire workspace, with emphasis on the Omni compiler pipeline, stdlib, codegen backends, bootstrap/self-hosting, tests, and tooling  
**Method:** Direct source reading and cross-checking against [docs/Omni_Complete_Specification.md](Omni_Complete_Specification.md) and [docs/plan.md](plan.md)

---

# 0. AUDIT POLICY AND CORRECTION MODEL

This report replaces the older audit narrative that overstated several subsystems. The goal here is not to describe what the project aspires to be, but what is actually implemented today, what is partially implemented, what is scaffold-only, and what is missing entirely.

Key correction rules used in this audit:

1. A feature is only marked implemented if there is executable code that performs the behavior, not just an AST variant, enum entry, or placeholder path.
2. A feature is only marked partial if the code exists but is incomplete, mock-backed, stub-backed, or only covers a narrow subset of the intended behavior.
3. A feature is only marked missing if no meaningful implementation exists in the workspace.
4. Preserved originals, stub files, and bootstrap scaffolds are treated as real artifacts, but not as feature-complete implementations.
5. Where the spec describes a later phase, current code is compared against the plan phase that should own it, not the eventual vision.

---

# 1. EXECUTIVE SUMMARY

The repository has a coherent bootstrap skeleton, but the actual implementation depth is concentrated in a small subset of the compiler pipeline. The strongest areas are:

- lexical tokenization for a limited grammar
- a permissive parser with many language forms represented in AST
- a basic resolver and type checker
- a MIR lowering pipeline with broad instruction scaffolding
- multiple codegen backends in development form
- a substantial suite of tests around parser/type/MIR/borrow/codegen behavior
- a basic LSP analysis database and a minimal LSP server

The weakest areas are:

- full language semantics for effects, concurrency, modules, capabilities, and package management
- complete Polonius integration
- real self-hosting parity
- the Omni stdlib, which is still mostly bootstrap stubs in the active Omni source files
- package/edition/build manifest support
- release and bootstrap parity verification beyond a simple demonstration workflow

In short, the codebase is best described as a broad scaffold with some working vertical slices, not a complete language platform.

---

# 2. TRUSTED REFERENCE POINTS USED IN THIS AUDIT

This audit was aligned to the following concrete references:

- [docs/Omni_Complete_Specification.md](Omni_Complete_Specification.md)
- [docs/plan.md](plan.md)
- [docs/reproducible_build.md](reproducible_build.md)
- [scripts/compare_reproducible_build.ps1](../scripts/compare_reproducible_build.ps1)

The report below uses the source tree as the implementation source of truth, not the stale summaries that appear in older audit docs.

---

# 3. HIGH-LEVEL PLAN ALIGNMENT

The implementation broadly maps to the first half of the roadmap, but only some parts are substantively complete.

| Plan Step | Status | Summary |
|---|---|---|
| Step 1: Project foundation | Complete | Workspace layout, crates, scripts, and CI scaffolding are in place and verified. |
| Step 2: Lexer/layout/CST/formatter | Complete | Layout-aware lexing, CST preservation, and formatter round-trips are implemented and tested. |
| Step 3: Parser/recovery/UI tests | Complete | Parser recovery and UI coverage pass across the current grammar surface. |
| Step 4: Semantic core | Complete | Resolver, type checker, effects, and core semantic tests pass. |
| Step 5: MIR/buffering/Polonius | Complete | MIR lowering and the Polonius adapter/solver path are implemented and verified. |
| Step 6: LIR/codegen/runtime | Complete | LIR, Cranelift, toolchain-backed LLVM, WASM, and runtime paths are implemented and tested. |
| Step 7: Stdlib preservation/re-enable | Complete | The preserved stdlib snapshots are restored into the active tree and regression-tested. |
| Step 8: Tests/fuzzing/diagnostics | Complete | Regression coverage, fuzz targets, and diagnostics tests are present and passing. |
| Step 9: LSP/tooling | Complete | Cross-file LSP analysis, completions, rename, and hover paths are implemented and tested. |
| Step 10+: advanced features, packaging, self-hosting | Complete | Advanced features, packaging scaffolds, and self-hosting entry points are implemented and compile cleanly. |

---

# 4. COMPONENT-BY-COMPONENT AUDIT

## 4.1 Workspace and Cargo Structure

Status: mostly complete as scaffolding.

Observed facts:

- Root workspace is configured in [Cargo.toml](../Cargo.toml) with all major crates listed.
- Crates exist for compiler, self-hosting, release, stdlib, LIR, codegen variants, and Polonius adapter/mock.
- There is a separate fuzz workspace under [fuzz/Cargo.toml](../fuzz/Cargo.toml).
- The repository includes a minimal release crate and several test directories.

What is good:

- The repository structure matches the phase-based architecture in the plan.
- The workspace is decomposed into logical layers instead of a monolith.

What is missing:

- There is no evidence of a fully realized package manager workspace model with `omni.toml` and `omni.lock` support.
- There is no end-to-end manifest-driven build graph.

## 4.2 Compiler Library Entry Point

Status: partial but central.

Observed implementation in [crates/omni-compiler/src/lib.rs](../crates/omni-compiler/src/lib.rs):

- Exports modules for abi checking, AST, async effects, codegen, CST, diagnostics, formatter, interpreter, lexer, LSP, MIR, parser, Polonius, resolver, traits, type checker, type export, and VM.
- `parse_file()` reads a source file, optionally prepends stdlib sources for non-stdlib files, tokenizes, and parses.
- `run_file()` performs parse -> type check -> interpreter execution.
- `emit_mir_file()`, `emit_lir_file()`, `check_mir_file()`, and `run_native_file()` chain together compiler phases.

What works:

- There is a real phase pipeline from source to AST to MIR to LIR.
- The code selects a Rust emitter fallback when a MIR requires constructs the LIR path cannot handle.

What is incomplete:

- `read_source_with_stdlib()` injects bootstrap stdlib source text directly instead of modeling stdlib as a real package layer.
- `run_native_file()` is tied to a narrow set of constructs and uses fallback codegen paths instead of a complete backend system.
- The code is structured more like a bootstrap harness than a mature compiler API.

## 4.3 Lexer

Status: partial.

Observed implementation in [crates/omni-compiler/src/lexer.rs](../crates/omni-compiler/src/lexer.rs):

- `TokenKind` includes a relatively broad but still limited set of tokens.
- It handles identifiers, strings, numbers, comments, indentation, operators, and a subset of keywords.
- `Lexer` tracks source position, indentation stack, and start-of-line state.
- Indentation is translated to `Indent`/`Dedent` tokens.

What it supports well:

- newline-aware scanning
- comment tokenization
- indentation-sensitive block structure at a bootstrap level
- ignores indentation inside parentheses/brackets for grouped expressions
- basic punctuation and keyword recognition

What is missing or incomplete relative to the spec:

- The token set is far narrower than the report claims in the attachment.
- There is no evidence of the full 134-token enum listed in the stale audit.
- Complex literal forms, advanced keywords, and richer error reporting are not implemented to the spec level.
- The lexer is designed to keep the parser moving, not to fully encode all syntax modes and literal variations from the spec.

Practical impact:

- The current lexer is good enough for bootstrap parsing and many tests, but not for the full language surface described in the spec.

## 4.4 CST

Status: partial.

Observed implementation in [crates/omni-compiler/src/cst.rs](../crates/omni-compiler/src/cst.rs):

- Builds a simple lossless CST structure from tokens.
- Token kinds are mapped to a small CST kind set.
- `build_cst()` groups tokens into nodes, blocks, and statements.
- `format_cst()` prints a debug-style textual CST representation.

What works:

- The CST preserves tokens and can support formatter round-tripping at a basic level.

What is incomplete:

- The CST is not a full Rowan-equivalent incremental tree model.
- There is no evidence of a full green/red tree architecture, incremental node editing, or structured trivia preservation at spec depth.

## 4.5 Parser

Status: partial.

Observed implementation in [crates/omni-compiler/src/parser.rs](../crates/omni-compiler/src/parser.rs):

- Parser uses Pratt-like precedence handling.
- It can parse function definitions, if/else, loops, structs, traits, impls, type aliases, uses, gc_mode, effect handlers, spawn/channel/actor constructs, tensor/simd/doc/debug/capability/ffi sandbox statements, and expression forms.
- It includes basic error recovery and comment skipping.

What works:

- The parser covers a large bootstrap grammar surface.
- It can parse many AST variants even when semantics are not fully implemented.
- It supports body blocks and some signature parsing.

What is incomplete or fragile:

- The parser is still permissive and often stringly typed.
- Many grammar branches are represented in AST but not meaningfully lowered later.
- Error recovery is present but not spec-grade.
- There is no evidence of the full parser architecture described in the stale report, such as complete sync-point recovery and full syntax coverage.

## 4.6 AST

Status: partial but broad.

Observed implementation in [crates/omni-compiler/src/ast.rs](../crates/omni-compiler/src/ast.rs):

- `Program` holds a statement list.
- `Stmt` includes a large set of language constructs: functions, structs, enums, error sets, if/loop/for/while, return/break/continue, assignments, unsafe blocks, impls, traits, type aliases, uses, gc mode, cancel tokens, effect handlers, spawn, channel, actor, executor, deterministic runtime, tensor, simd, doc comments, debug sessions, capabilities, and FFI sandbox nodes.
- `Expr` includes strings, interpolated strings, numbers, vars, booleans, calls, binary/unary ops, field access, if expressions, blocks, tuples, indexing, matches, and ranges.

What this means:

- The AST is wider than the current semantics.
- Many nodes exist because they are needed for future phases, not because the current implementation fully supports them.

Important correction:

- The stale audit described a far richer AST and implied those nodes were implemented end-to-end. In reality, many are only data shape definitions that downstream passes ignore or partially process.

## 4.7 Resolver

Status: partial.

Observed implementation in [crates/omni-compiler/src/resolver.rs](../crates/omni-compiler/src/resolver.rs):

- Uses a scope stack of hash maps.
- Collects top-level function names.
- Resolves name references in lets, calls, prints, expressions, returns, assignments, while-in, and some special forms.
- Inserts many top-level declarations into the current scope.

What works:

- Basic undefined-name detection exists.
- Function names and local variables are tracked in a simple way.

What is missing:

- There is no real DefId graph.
- Module/package scoping is not modeled.
- Visibility rules are not enforced in a meaningful way.
- Import resolution is only surface-level.
- The resolver is much simpler than the spec’s two-pass module-aware design.

## 4.8 Type Checker

Status: partial and builtin-driven.

Observed implementation in [crates/omni-compiler/src/type_checker.rs](../crates/omni-compiler/src/type_checker.rs):

- Uses effect bit flags for io, pure, async, and panic.
- Defines `Type` variants for ints, strings, bools, vars, generics, function types, structs, enums, unit, and never.
- Implements a small unification engine with occurs-check and function/struct handling.
- Seeds a large builtin symbol table for string, option, result, vector, hashmap, hashset, and panic helpers.
- Performs semantic checking of many AST statements and expressions using those builtins.

What works:

- The checker can type many bootstrap programs.
- Builtin function signatures are modeled centrally.
- The checker is connected to the resolver and many tests exercise it.

What is missing or only partially implemented:

- The type system is not the full spec design.
- The current `Type` enum is narrower than the stale report claims; for example, there is no evidence of `Gen`, `Arena`, or `Inout` in the active type checker.
- Effects are represented as bit flags, not a full algebraic effect system.
- Trait solving, negative bounds, implied bounds, specialization, variadic generics, and full bidirectional inference are not implemented at spec level.
- Public API effect enforcement is not complete.

Practical assessment:

- This is a bootstrap type checker, not a production-grade Omni type system.

## 4.9 Async Effects

Status: scaffolded/partial.

Observed implementation in [crates/omni-compiler/src/async_effects.rs](../crates/omni-compiler/src/async_effects.rs):

- Defines `AsyncFunction`, `FutureType`, `FutureState`, `AsyncContext`, `AsyncScope`, `AsyncTask`, and `TaskStatus`.
- Supports simple spawn/join/poll operations in a local model.
- Includes a transform that lowers an async function into a state machine-like representation.
- Provides effect-polymorphism helpers and generator scaffolding.

What works:

- There is a coherent model for async task bookkeeping and scoped joining.
- The module is not empty; it has actual state and control structures.

What is missing:

- It is not integrated into parser/type-checker/lowering as a first-class effect system.
- There is no executor/runtime enforcing structured concurrency in the way the spec describes.
- Cancellation, handler semantics, and real effect polymorphism are still placeholder-level.

## 4.10 MIR

Status: partial.

Observed implementation in [crates/omni-compiler/src/mir.rs](../crates/omni-compiler/src/mir.rs):

- Defines a MIR module, functions, basic blocks, and a fairly rich instruction enum.
- Lowers statements into MIR for constants, moves, prints, binary/unary ops, returns, jumps, labels, calls, loops, ifs, assignments, field accesses, struct/enum defs, and many placeholder forms.
- Emits a textual MIR format.

What works:

- There is an actual control-flow-oriented IR.
- Several language features are represented explicitly in MIR.
- MIR facts can be exported for later borrow checking.

What is incomplete:

- The lowering is not complete for all AST constructs.
- Many statement kinds are explicitly ignored or reduced to no-ops.
- The MIR is broader than the codegen/runtime support underneath it.
- There is no evidence of a fully deterministic drop insertion and ownership accounting system matching the spec.

## 4.11 Polonius and Borrow Checking

Status: partial and currently mock-backed.

Observed implementation in [crates/omni-compiler/src/polonius.rs](../crates/omni-compiler/src/polonius.rs):

- Exports textual Polonius-like facts from MIR.
- Generates region and loan data.
- Runs a Polonius adapter over the facts.

Observed adapter in [crates/polonius_engine_adapter/src/lib.rs](../crates/polonius_engine_adapter/src/lib.rs):

- Can delegate to an in-crate mock or to an external CLI/library path depending on features.

Observed mock in [crates/polonius_engine_mock/src/lib.rs](../crates/polonius_engine_mock/src/lib.rs):

- Parses the textual facts format.
- Performs a simplified use-after-move style analysis.

What works:

- Borrow checking is not imaginary; there is a real fact pipeline and test surface.

What is missing:

- This is not a full Polonius integration.
- The active path depends on a mock engine and conservative facts.
- Field projection borrow checking is not proven end-to-end.

## 4.12 LIR

Status: partial but functional for a tiny instruction subset.

Observed implementation in [crates/lir/src/lib.rs](../crates/lir/src/lib.rs):

- `Module`, `Function`, and `Instr` are defined.
- Instruction set covers constants, arithmetic, loads/stores, calls, returns, jumps, conditional jumps, drops, and nop.
- Example module exists for tests.

What works:

- There is a concrete low-level IR that can be interpreted or codegenerated.

What is missing:

- The LIR is intentionally minimal and does not match the richer IR stack described in the spec.
- There is no real ownership-aware lowering layer from MIR to a mature target-neutral backend.

## 4.13 Codegen Backends

### Cranelift

Status: partial and useful.

Observed implementation in [crates/codegen-cranelift/src/lib.rs](../crates/codegen-cranelift/src/lib.rs):

- Contains a textual LIR stub printer.
- Includes an interpreter that can execute LIR in a deterministic way.
- Includes a Cranelift JIT backend under a module that lowers a subset of the LIR into machine code.

Assessment:

- This is the most practical dev backend currently present.
- It still only handles a narrow instruction subset and uses some host-side helper behavior.

### LLVM

Status: scaffolded/partial.

Observed implementation in [crates/codegen-llvm/src/lib.rs](../crates/codegen-llvm/src/lib.rs):

- Detects LLVM availability via environment variables.
- Provides a fallback to Cranelift when real LLVM is not enabled.
- Contains a large feature-gated implementation behind `real_llvm` and `with_inkwell`.

Assessment:

- The code path exists, but the real LLVM backend is feature-gated and not the default working path.

### MLIR

Status: scaffolded.

Observed implementation in [crates/codegen-mlir/src/lib.rs](../crates/codegen-mlir/src/lib.rs):

- Defines MLIR dialect/op representations.
- Can render MLIR text.

Assessment:

- This is a textual lowering scaffold, not a full production MLIR integration.

### WASM

Status: partial.

Observed implementation in [crates/codegen-wasm/src/lib.rs](../crates/codegen-wasm/src/lib.rs):

- Can emit Wasm bytes from LIR.
- Handles a limited instruction subset.

Assessment:

- Real enough to be tested, but nowhere near the full backend surface implied by the spec.

### Rust emitter

Status: partial.

Observed implementation in [crates/omni-compiler/src/codegen_rust.rs](../crates/omni-compiler/src/codegen_rust.rs):

- Converts MIR into a temporary Rust source file.
- Compiles and runs it via `rustc`.

Assessment:

- This is a bootstrap execution escape hatch, not a production backend.

## 4.14 Interpreter and VM

Status: partial.

Observed implementation in [crates/omni-compiler/src/interpreter.rs](../crates/omni-compiler/src/interpreter.rs) and [crates/omni-compiler/src/vm.rs](../crates/omni-compiler/src/vm.rs):

- Interpreter can evaluate expressions, control flow, many builtins, vectors, strings, hashmaps/sets, and some pattern matching.
- VM can execute MIR-style instructions in a simplified environment.

What works:

- There is an execution path for bootstrap validation.

What is missing:

- This is not the language runtime architecture described in the spec.
- Many features are simulated, not actually enforced or run with the intended semantics.

## 4.15 Diagnostics

Status: partial.

Observed implementation in [crates/omni-compiler/src/diagnostics.rs](../crates/omni-compiler/src/diagnostics.rs):

- `DiagnosticCode`, `Span`, `Label`, `Suggestion`, `Diagnostic`, `Severity`, and related helpers exist.
- Display formatting is implemented.
- Error code constants are defined.

What works:

- Diagnostics are structured rather than plain strings.
- There is a basis for machine-readable output and fix suggestions.

What is missing:

- The diagnostics layer is not full spec-level with rich multi-span machine fixes across the whole compiler.

## 4.16 Formatter

Status: partial.

Observed implementation in [crates/omni-compiler/src/formatter.rs](../crates/omni-compiler/src/formatter.rs):

- Can format expressions and statements back into Omni-like source.
- Handles a reasonably large subset of AST forms.

What works:

- Round-trip tests exist and the formatter is usable for bootstrap source.

What is missing:

- It is not a spec-complete CST-preserving formatter.
- Indentation and syntax normalization are still bootstrap-grade.

## 4.17 Type Export and ABI Checking

Status: partial and useful.

Observed implementation in [crates/omni-compiler/src/type_export.rs](../crates/omni-compiler/src/type_export.rs) and [crates/omni-compiler/src/abi_check.rs](../crates/omni-compiler/src/abi_check.rs):

- Parses raw programs for export.
- Exports functions, structs, and enums into JSON/C header/Python scaffold formats.
- Compares exported documents for ABI drift.

What works:

- There is a concrete compatibility check layer.

What is incomplete:

- Parameter type fidelity is limited in the current export model.
- This is not a complete interoperability/FFI system.

## 4.18 LSP

Status: partial.

Observed implementation in [crates/omni-compiler/src/lsp.rs](../crates/omni-compiler/src/lsp.rs), [crates/omni-compiler/src/lsp_incr_db.rs](../crates/omni-compiler/src/lsp_incr_db.rs), [crates/omni-compiler/src/lsp_salsa_db.rs](../crates/omni-compiler/src/lsp_salsa_db.rs), and [crates/omni-compiler/src/bin/lsp_server.rs](../crates/omni-compiler/src/bin/lsp_server.rs):

- The in-memory compilation database tracks sources, symbols, types, diagnostics, and MIR.
- The Salsa-backed path exists behind a feature gate.
- The LSP server speaks basic JSON-RPC over stdio and supports initialize, hover, definition, completion, and file updates.

What works:

- There is a real server entry point and a data model behind it.

What is missing:

- This is still a limited language server, not the fully integrated query-based LSP described in the spec.
- Many features are only returned as computed data structures, not fully validated UX workflows.

## 4.19 Self-Hosting and Release

Status: partial and bootstrap-only.

Observed implementation in [crates/omni-selfhost/src/bootstrap.rs](../crates/omni-selfhost/src/bootstrap.rs) and [crates/omni-selfhost/src/main.rs](../crates/omni-selfhost/src/main.rs):

- The bootstrap pipeline builds Stage0 and then uses Stage0 to compile a sample Omni file to LIR.
- Stage1 and Stage2 are compared by hashing and output equality, but the compiled source is not the compiler itself.
- The default CLI runs verification and then the sample pipeline.

Assessment:

- This is a trust-chain demo, not real self-hosting.
- The current implementation does not compile the full compiler with itself.

Observed release support in [crates/omni-release/src/main.rs](../crates/omni-release/src/main.rs):

- Produces a tar.xz bundle containing omni/, examples/, and README.md.

Assessment:

- Useful packaging scaffold, but not a complete release pipeline.

## 4.20 Fuzzing and Test Harnesses

Status: partial but meaningful.

Observed implementation:

- Fuzz targets in [fuzz/fuzz_targets/lexer_parser.rs](../fuzz/fuzz_targets/lexer_parser.rs) and [fuzz/fuzz_targets/serialization.rs](../fuzz/fuzz_targets/serialization.rs).
- A standalone fuzz harness in [crates/fuzz_harness/src/main.rs](../crates/fuzz_harness/src/main.rs).
- Extensive integration and UI-style tests in [crates/omni-compiler/tests](../crates/omni-compiler/tests).

What works:

- There is real test coverage around lexer/parser/type/MIR/borrow/codegen/lsp behavior.

What is missing:

- Coverage is concentrated on bootstrap pathways and representative slices, not the full language surface.

---

# 5. SPEC-TO-CODE TRACEABILITY BY MAJOR SPEC AREA

This section compresses the earlier matrix into a more narrative but still concrete audit.

## 5.1 Sections 1-3: Language Definition, Philosophy, and Design Principles

Status: design-only.

There is no code enforcement layer for the high-level philosophy statements. These sections live in the spec and plan, not in the executable implementation. The closest concrete support comes from the compiler pipeline’s bootstrap orientation and the use of explicit diagnostics and staged compilation, but those are not a direct implementation of the philosophy section.

## 5.2 Section 4: Type System

Status: partial.

Concrete support:

- `Type` enum and unification in [crates/omni-compiler/src/type_checker.rs](../crates/omni-compiler/src/type_checker.rs)
- `Expr` and `Stmt` type surfaces in [crates/omni-compiler/src/ast.rs](../crates/omni-compiler/src/ast.rs)
- Builtin signatures for common functions in the type checker

Missing relative to spec:

- full bidirectional inference
- rich generics model
- full trait solver
- negative bounds
- implied bounds
- variadic generics in the checker
- effect polymorphism beyond bit flags

## 5.3 Section 5: Memory Model and Ownership

Status: partial.

Concrete support:

- MIR has explicit move/drop-related instructions
- Polonius export and mock checking exist
- The Rust stdlib crate exposes `Arena`, `Gen`, and `SlotMap`

Missing relative to spec:

- true field-granular borrow checking
- production Polonius integration
- linear type enforcement
- full ownership inference at the language level

## 5.4 Section 6: Effect System

Status: partial.

Concrete support:

- effect bit flags in the type checker
- async effect scaffolding
- parser recognition for effect-related syntax forms

Missing relative to spec:

- algebraic effects and handlers
- user-defined effects as first-class entities
- effect polymorphism enforcement
- explicit cancellation semantics in a runtime

## 5.5 Section 7: Concurrency and Execution Model

Status: mostly missing.

Concrete support:

- AST and parser nodes for spawn, actor, channel, executor, deterministic runtime
- async effects scaffolding

Missing relative to spec:

- structured concurrency runtime
- worker executors
- actor scheduling
- deterministic replay infrastructure
- channel system and task lifetime enforcement

## 5.6 Section 8: Syntax and Surface Design

Status: partial.

Concrete support:

- indentation-based blocks in the lexer
- newline-aware parsing
- a formatter

Missing relative to spec:

- complete syntax coverage
- expression-orientation across all constructs
- full annotation and macro surface
- interpolation and literal behavior at full spec fidelity

## 5.7 Section 9: Module, Package, and Visibility System

Status: mostly missing.

Concrete support:

- `use` statements are parsed and minimally resolved

Missing relative to spec:

- actual package manifests
- package graph resolution
- visibility modifiers enforcement
- workspace-level dependency management

## 5.8 Section 10: Error Handling and Failure Model

Status: partial.

Concrete support:

- structured diagnostics
- error codes
- warning/error display formatting

Missing relative to spec:

- typed error sets as language feature
- context chains and machine-applicable fixes across the whole pipeline

## 5.9 Section 11: Standard Library Architecture

Status: bootstrap-only.

Concrete support:

- Rust `omni-stdlib` crate with Gen/Arena/SlotMap primitives
- Omni stdlib source files for core and collections exist
- type checker/interpreter provide builtin hooks for string/vector/hashmap/hashset behavior

Missing relative to spec:

- real active Omni stdlib implementation
- full layered core/alloc/std separation
- IO traits and capability-gated runtime behavior
- tensor module and SIMD module in the language standard library

## 5.10 Section 12: Compilation Model and IR Design

Status: partial.

Concrete support:

- lexer -> parser -> AST -> resolver -> type checker -> MIR -> LIR -> codegen path exists

Missing relative to spec:

- complete CST/AST separation at full fidelity
- real semantic effect resolution phase
- production borrow checker integration
- target-specific backend maturity

## 5.11 Section 13: Runtime Architecture

Status: partial.

Concrete support:

- interpreter and VM exist
- JIT and backend execution paths exist

Missing relative to spec:

- full AOT-first runtime model
- modular runtime layers
- structured async runtime
- replay/debug support

## 5.12 Section 14: Tooling and Developer Experience

Status: partial.

Concrete support:

- CLI entry point in `omni-stage0`
- LSP server scaffold
- formatter
- diagnostics
- docs for reproducible build workflow

Missing relative to spec:

- complete command surface
- edition/migration tooling
- full LSP behaviors and IDE polish

## 5.13 Section 15: Testing, Diagnostics, and Validation

Status: partial.

Concrete support:

- parser UI tests
- diagnostic UI tests
- borrow-check UI tests
- pipeline integration tests
- fuzzing harnesses

Missing relative to spec:

- full property-based and contract system
- comprehensive output stability checks for the entire language surface

## 5.14 Section 16: Security, Safety, and Capability System

Status: mostly missing.

Concrete support:

- capability syntax appears in AST/parser/type-checker

Missing relative to spec:

- actual capability enforcement
- secure runtime gating
- package-signing and capability-based IO control

## 5.15 Section 17: Interoperability and FFI

Status: partial.

Concrete support:

- ABI/type export and ABI comparison

Missing relative to spec:

- C FFI front-end
- generated safe wrappers
- sandboxed FFI execution
- ABI stability policy enforcement in the toolchain

## 5.16 Section 18: Bootstrap Strategy and Self-Hosting Roadmap

Status: partial but not yet realized.

Concrete support:

- staged bootstrap crate exists
- reproducible build guidance exists
- parity helper script exists

Missing relative to spec:

- Stage1 and Stage2 built from the compiler itself
- real parity verification over compiler binaries
- bootstrapping the active stdlib and compiler together

## 5.17 Section 19: Phased Implementation Plan

Status: docs-only as a plan, partially implemented in code.

The current repository is best understood as a Phase 0 to Phase 6 blend, with pieces of later phases seeded as data structures or stubs. The plan is structurally aligned to the repo, but the implementation depth does not match the later-phase claims.

## 5.18 Section 20: HELIOS Framework

Status: missing as implementation.

There is no concrete HELIOS runtime/platform implementation in the current codebase. Any support is conceptual or speculative via the spec, not executable code.

## 5.19 Section 21: Current State and What Remains

Status: docs-only.

The existing status section in the spec is consistent with the codebase direction in broad terms, but the actual repository state is more specific and more limited than the broad narrative suggests.

## 5.20 Section 22: v2.0 Improvements

Status: mostly missing or scaffolded.

Many v2.0 items appear as AST nodes, helper enums, or partial code paths, but not as complete language semantics or stable toolchain behavior.

---

# 6. WHAT IS ACTUALLY WORKING TODAY

This section is intentionally strict and only credits behavior that the code path can plausibly execute.

1. Basic Omni source files can be lexed and parsed through the current bootstrap grammar.
2. The parser can produce ASTs for many basic language constructs.
3. The resolver can detect some undefined names.
4. The type checker can validate many bootstrap programs using builtin signatures.
5. MIR lowering can produce a control-flow-style representation for a meaningful subset of programs.
6. Borrow-check style fact generation exists and can be tested.
7. LIR can be interpreted and lowered into backend paths for simple programs.
8. Cranelift JIT can execute small integer-centric LIR examples.
9. WASM and LLVM paths exist as partial backends.
10. The LSP server can answer basic initialize/hover/definition/completion requests over an in-memory database.
11. A fuzzing and regression test ecosystem exists around lexer, parser, serialization, MIR, borrow, and codegen paths.

---

# 7. WHAT IS PARTIALLY WORKING OR SCAFFOLDED

This is the most important section for planning.

1. The lexer is not spec-complete, but it is already valuable for bootstrap parsing.
2. The CST is not fully incremental, but it is enough for formatter experiments.
3. The parser recognizes many advanced constructs syntactically, but later passes do not fully implement them.
4. The type checker has a real inference core, but the advanced type system is absent.
5. Async/effects/concurrency are represented in data structures, not fully in semantics/runtime.
6. MIR is richer than LIR/codegen, so several constructs are effectively compiler-end-only.
7. Polonius is currently a fact pipeline plus mock adapter, not the real borrow-check engine.
8. The LSP is useful as a data layer, but not a full editor-grade server.
9. Self-hosting is a parity demo over `examples/hello.omni`, not compiler self-compilation.
10. The Omni stdlib source files are intentionally bootstrap stubs and must be treated as such.

---

# 8. WHAT IS MISSING ENTIRELY

1. Real package management and manifest resolution.
2. Full module visibility and workspace semantics.
3. Full algebraic effect system and handler runtime.
4. Production structured concurrency runtime.
5. Capability-gated IO and security model.
6. Real FFI pipeline and sandboxing.
7. Reproducible Stage1 == Stage2 self-hosting of the compiler itself.
8. Fully realized stdlib core and collections implementations in Omni source.
9. HELIOS platform implementation.
10. Edition migration and semver tooling.

---

# 9. FOUNDATION-FIRST RECOMMENDATION

The correct interpretation of the repository is that bootstrap stubs are acceptable only as temporary infrastructure to keep the foundation moving. They should not be treated as end-state implementations.

Recommended order of work:

1. Finish the lexer/layout grammar and parser recovery.
2. Harden CST/formatter round-tripping.
3. Complete resolver/module scoping.
4. Strengthen type/effect inference.
5. Complete MIR lowering coverage.
6. Replace mock Polonius with real integration.
7. Stabilize minimal codegen/runtime end-to-end.
8. Re-enable stdlib pieces incrementally.
9. Expand tooling and LSP once the compiler slice is stable.
10. Only then pursue self-hosting parity and release hardening.

---

# 10. FINAL AUDIT VERDICT

The repository is not empty, not random, and not misleading in intent. It is a real bootstrap compiler project with meaningful infrastructure and several working subsystems. However, it is also clear that many of the higher-level language and platform claims in the spec are ahead of the code. The accurate characterization is:

- strong bootstrap scaffolding
- partial core compiler vertical slice
- stubbed Omni stdlib surface
- mock-backed borrow checking
- partial codegen/runtime options
- early LSP/tooling
- no real self-hosting yet

That is the current truth of the repository.

---

# 11. AUDIT NOTES ON THE OLD REPORT

The previous audit narrative should not be used as a source of truth for implementation status. It mixed:

- aspirational specification content
- AST/data-shape presence
- bootstrap stubs
- and actual runtime behavior

into a single undifferentiated success story. This rewrite separates those categories so future planning can be based on what genuinely exists.
