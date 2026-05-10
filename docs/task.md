# Omni Compiler Remediation - Task Tracker

## Phase 1: Dead Code Removal ✅
- [x] Delete `lexer.rs`, `complete_parser.rs`, `phase1_bridge.rs`
- [x] Purge 18 stale root files (debug scripts, temp files)
- [x] Remove `pub mod lexer;` from `lib.rs`
- [x] Migrate 9 files from legacy `Lexer` to `complete_lexer::tokenize_complete`
- [x] Fix double-unwrap in `layout_edge_cases.rs`
- [x] Verify `cargo check --workspace` passes

## Phase 2: Module Unification ✅
- [x] Delete orphan `bidirectional_typer.rs` (not declared in lib.rs)
- [x] Delete orphan `effect_system.rs` (not declared in lib.rs)
- [x] Delete orphan `trait_system.rs` (not declared in lib.rs)
- [x] Delete orphan `levenshtein.rs` (not declared in lib.rs)
- [x] Keep `generational_refs` (declared in lib.rs, may be used externally)
- [x] Verify `cargo check --workspace` passes

## Phase 3-4: Deferred
- [ ] ScopeTree resolver integration (requires full validation)
- [ ] AST span hardening (requires coordinated AST + parser changes)

## Phase 5: Pipeline Wiring & Critical Bug Fixes ✅
- [x] Wire `resolver::resolve_program()` into all pipeline functions in `lib.rs`:
  - `run_file`, `check_file`, `emit_mir_file`, `check_mir_file`, `run_native_file`
- [x] Fix double-unwrap in 4 test files: `advanced_features.rs`, `borrow_check_ui.rs`, `diagnostic_ui.rs`, `type_inference_ui.rs`
- [x] Fix corrupted import `complete_complete_lexer` in `mir_optimize.rs`, `codegen_lir.rs`, `advanced_features.rs`
- [x] Fix garbled tokenization in `debug_tokens.rs` and `public_api_effects.rs`
- [x] **CRITICAL BUG FIX: Parser keyword token dispatch**
  - Parser only checked `TokenKind::Ident` with text matching for keywords
  - Added dedicated handlers for `Let`, `Fn`, `Return`, `For`, `While`, `Loop`, `Struct`, `Impl`, `Trait`, `Type`, `Use`, `Break`, `Continue`, `Spawn`
  - Added `TokenKind::Pub` and `TokenKind::Async` to prefix effect handling
- [x] **CRITICAL BUG FIX: Lexer stale-cursor bug after indent processing**
  - After `indent_of()` consumed whitespace, the loop variable `c` was stale
  - Added `continue` after indent block to restart with fresh `peek_char()`
- [x] **CRITICAL BUG FIX: Lexer raw string prefix `r` greedy consumption**
  - `if c == 'r'` consumed 'r' before checking if '#' follows
  - This broke all identifiers/keywords starting with 'r' (return, ref, etc.)
  - Fix: Use `peek_nChar(1)` lookahead before consuming
- [x] **CRITICAL BUG FIX: Lexer byte string prefix `b` greedy consumption**
  - Same issue as `r` prefix — fixed with `peek_nChar(1)` guard
- [x] **Parser enhancement: Curly brace `{ }` block support**
  - Function body parser only accepted `[`/`]` and indent blocks
  - Added `LBrace`/`RBrace` as valid block delimiters
  - Fixed 50 failing generated regression tests
- [x] Verify: 200/200 generated regression tests pass
- [x] Verify: All borrow_check_ui tests pass (6/6)
- [x] Verify: Full workspace builds and 1 pre-existing failure remains (block_comments_preserved in layout_edge_cases)

## Phase 6: MIR Lowering Completion ✅
- [x] Function calls with arguments - properly lowers Call expressions with args
- [x] Struct field access - added FieldAccess handling in MIR lowering
- [x] String operations - basic string literals already handled
- [x] Match expressions - added MIR lowering with MatchBranch instruction
- [x] Verified function calls compile through full pipeline (parser → resolver → type checker → MIR → borrow check)
- [x] Added MatchBranch instruction to MIR, codegen_rust, polonius, vm, and format_mir

## Phase 7: Trait & Effect Integration ✅
- [x] Effect system already implemented in type_checker.rs
- [x] Effect tracking during type checking with u8 bitmask
- [x] Effects properly propagated through function calls and expressions
- [x] Public function effect annotation enforcement

## Phase 8: End-to-End Validation ✅
- [x] Hello World test passes: `print "Hello, Omni!"` → native execution
- [x] Pipeline integration tests pass (5/5 tests)
- [x] Full pipeline: parse → resolve → typecheck → MIR → borrow check → LIR → native
- [x] 200/200 generated regression tests pass
- [x] 7/7 borrow_check_ui tests pass
- [ ] 1 pre-existing failure remains (block_comments_preserved in layout_edge_cases)


## Phase 9: Syntax & Type System Completeness ✅
- [x] Add support for Float and Char types in AST, lexer, parser, and type checker.
- [x] Add parsing for Pipe operator (`|>`)
- [x] Implement Type Aliases parsing and resolution (`type Alias = Target`).
- [x] Implement Refinement Types.

## Phase 10: Effect System & Generics ✅
- [x] Implement the `performs` syntax for effect annotations.
- [x] Connect Effect handlers AST nodes to semantic analysis and runtime semantics.
- [x] Implement Generics instantiation and Effect inference/polymorphism.

## Phase 11: Memory Model & Concurrency ✅
- [x] Implement Generational references and Arena allocation.
- [x] Connect structured concurrency `SpawnScope` types to an actual runtime system/codegen.
- [x] Implement proper Async/await codegen.
- [x] Implement the Actor model from AST to implementation.

## Phase 12: Advanced Compilation & Tooling ✅
- [x] Complete the LLVM backend (currently C-emission only).
- [x] Implement WebAssembly (WASM) target.
- [x] Implement Comptime (Zig-style) compile-time execution.
