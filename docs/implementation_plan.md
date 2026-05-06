# Omni Compiler — Implementation Plan (Post-Audit)

## Goal

Remediate the Omni compiler codebase from its current state (functional vertical slice with significant architectural debt) to a clean, spec-aligned foundation where `omni build hello.omni` produces a working binary through a fully-wired pipeline.

> [!IMPORTANT]
> This plan is derived directly from the exhaustive audit (Parts 1 & 2). Every action item maps to a specific audit finding. No assumptions are made.

---

## Phase 1: Dead Code Removal & Cleanup
**Goal:** Remove all legacy/stale code. Zero compilation ambiguity.

### [DELETE] `crates/omni-compiler/src/lexer.rs`
- 19,964 bytes of deprecated lexer. `complete_lexer.rs` is the canonical lexer.

### [DELETE] `crates/omni-compiler/src/complete_parser.rs`
- 145 lines importing `lexer::Token` (the deleted module). Compilation-breaking dependency.

### [DELETE] `crates/omni-compiler/src/phase1_bridge.rs`
- 160 lines of identity mapping. Parser now uses `complete_lexer` directly — this bridge is redundant.

### [MODIFY] `crates/omni-compiler/src/lib.rs`
- Remove `pub mod lexer;`, `pub mod complete_parser;`, `pub mod phase1_bridge;` declarations.

### [CLEANUP] Root directory stale files
- Delete: `debug_direct.rs`, `debug_lexer.rs`, `debug_parse.rs`, `debug_tokens.rs`, `test_tokenize.rs`, `test_tokenize_temp.rs`, `temp_type_checker.rs`, `fix.py`, `fix_tests.py`, `fix_tests_3.py`, `fix_tests_again.py`, `replace.py`, `check_errors.txt`, `check_errors_utf8.txt`, `last_mir_output.txt`, `test_output.txt`, `nul`, `debug_lexer.pdb`, `debug_lexer.*.o`, `graph.json`

### Verification
- `cargo check --workspace` passes cleanly.

---

## Phase 2: Module Unification
**Goal:** Eliminate overlapping implementations. One module per concern.

### 2A: Type System Unification
**Keep:** `type_checker.rs` (1,687 lines — more complete)
**Merge into it:** Any unique logic from `bidirectional_typer.rs` (410 lines)
**Delete:** `bidirectional_typer.rs` after merge

### 2B: Effect System Unification
**Keep:** `async_effects.rs` (317 lines — richer `Effect` enum with `Throw(Type)`, `Custom(String)`)
**Merge into it:** Bitmask constants from `effect_system.rs` (for backward compat with MIR)
**Rename:** `async_effects.rs` → `effect_system.rs` (overwrite the old one)

### 2C: Trait System Unification
**Keep:** `traits.rs` (10,515 bytes — has `TraitRegistry` with impl checking)
**Merge into it:** Negative bounds logic from `trait_system.rs` (255 lines)
**Delete:** `trait_system.rs` after merge

### 2D: Generational Refs Deduplication
**Keep:** `omni-stdlib` crate as the canonical location for `Gen<T>`, `Arena<T>`
**Modify:** `generational_refs.rs` to re-export from `omni-stdlib` instead of duplicating

### Verification
- `cargo check --workspace` passes.
- All existing tests still pass.

---

## Phase 3: ScopeTree Resolver Integration
**Goal:** Replace the legacy stack-of-HashMaps resolver with the ScopeTree architecture.

### [MOVE] `crates/resolver.absolute` → `crates/omni-compiler/src/resolver.rs`
- Fix compilation errors: add `Ordering::Relaxed` to `fetch_add`, fix function naming conventions
- Fix `resolve_Stmt` signature (takes `&[Stmt]` not `&Stmt` for the top-level call)
- Make `ScopeTree.scopes` field `pub(crate)` for test access only

### [MODIFY] `crates/omni-compiler/src/lib.rs`
- Wire `resolver::resolve_program()` into the pipeline between parsing and type checking
- Pass `ResolveResult` to type checker

### Verification
- Existing programs still parse and type-check.
- New test: undefined variable produces error with scope information.
- New test: shadowing resolves correctly.

---

## Phase 4: AST Hardening
**Goal:** Make the AST capable of supporting the full spec requirements.

### 4A: Add Spans to AST
- Add `Span { line: u32, col: u32, len: u32 }` to every `Stmt` and `Expr` variant
- Propagate span from `Token` during parsing

### 4B: Add Typed Parameters
- Change `Fn.params` from `Vec<String>` to `Vec<(String, Option<String>)>` (name + optional type annotation)
- Update parser to parse `name: type` parameter syntax
- Update all downstream consumers (type_checker, mir, formatter, interpreter)

### 4C: Add Missing Expr Variants
- `Expr::Lambda { params, body }` — closures
- `Expr::Await(Box<Expr>)` — async expressions
- `Expr::Try(Box<Expr>)` — `?` operator
- `Expr::StructLit { name, fields }` — struct construction

### Verification
- Parser roundtrip tests pass with spans.
- Type checker uses typed params for inference.

---

## Phase 5: Pipeline Wiring
**Goal:** All semantic passes connected in the correct order.

### Pipeline Order (matching §12.1):
```
Source → complete_lexer → Parser → AST
  → resolver::resolve_program() → ResolveResult
  → type_checker::check_program() → Typed AST
  → mir::lower_program_to_mir() → MIR
  → polonius::check_borrows() → Verified MIR
  → mir_optimize::optimize() → Optimized MIR
  → codegen_lir::lower_to_lir() → LIR
  → codegen::compile_and_run() → Output
```

### [MODIFY] `lib.rs`
- Add resolver step
- Add polonius step (currently skipped in main path)
- Add mir_optimize step
- Wire diagnostics: convert `String` errors to `Diagnostic` at each boundary

### [MODIFY] `diagnostics.rs`
- Add `from_string(msg, span)` helper for gradual migration
- Add JSON output mode (`emit_diagnostic_json`)

### Verification
- `cargo test` — all existing tests pass.
- New integration test: `compile_and_run_file("examples/hello.omni")` succeeds end-to-end.

---

## Phase 6: MIR Lowering Completion
**Goal:** All AST node types lower to MIR correctly.

### Currently missing lowering for:
- Function calls with arguments (only name-based, no arg passing)
- Match expressions (only simple `if/else` path)
- Struct construction and field access
- Closures/lambdas
- Tuple expressions
- Index expressions
- String operations beyond literals

### Priority order:
1. Function calls with arguments → enables real programs
2. Struct construction + field access → enables data structures
3. Match expressions → enables pattern matching
4. String operations → enables practical IO

### Verification
- fibonacci compiles and runs through full pipeline (not just interpreter).
- Struct creation + field access compiles through full pipeline.

---

## Phase 7: Trait & Effect Integration
**Goal:** Traits and effects enforced during type checking.

### 7A: Trait Integration
- Wire `TraitRegistry` into `InferCtx`
- Enforce trait bounds during unification
- Add `Send`/`Sync` marker traits for concurrency checking

### 7B: Effect Integration
- Add effect tracking to type checker's function analysis
- Infer effects from function body (propagate upward)
- Enforce: public functions must explicitly annotate effects

### Verification
- Test: calling IO function from pure context produces error.
- Test: trait bound violation produces error with suggestion.

---

## Phase 8: End-to-End Validation
**Goal:** The "Hello, World!" acceptance criterion from §21.3.

### Test programs that must work:
1. `print "Hello, World!"` → Cranelift JIT → output
2. `let x = 42; print x` → full pipeline → output
3. `fn add(a, b) -> int: return a + b; print add(1, 2)` → output "3"
4. `fn fib(n): if n <= 1: return n; return fib(n-1) + fib(n-2); print fib(10)` → output "55"
5. Same programs → WASM backend → output
6. Same programs → LLVM backend → output

### Verification
- CI test suite with all 6 programs × 3 backends = 18 passing tests.

---

## Open Questions

> [!IMPORTANT]
> **Q1:** Should we keep both `interpreter.rs` (AST-based) and `vm.rs` (MIR-based) execution paths, or consolidate to MIR-only?

> [!IMPORTANT]
> **Q2:** The spec says CST should be Rowan-based (§12.2, Appendix B). The current `cst.rs` is custom. Should we migrate to Rowan now, or defer to a later phase?

> [!IMPORTANT]
> **Q3:** `codegen-llvm` currently emits C and compiles with clang. Should we migrate to `inkwell` (LLVM bindings) now, or is C emission acceptable for this remediation?

---

## Verification Plan

### Automated Tests
- `cargo check --workspace` — no compilation errors
- `cargo test --workspace` — all tests pass
- `cargo clippy --workspace` — no warnings
- Integration tests for the 6 programs above

### Manual Verification
- Run `omni build examples/hello.omni` and verify binary output
- Inspect diagnostic output for type errors — verify spans are correct
