# GRAPHIFY KNOWLEDGE GRAPH - OMNI CODEBASE

## Graph Summary

| Metric | Count |
|--------|-------|
| Total Nodes | 929 |
| Total Edges | 1928 |
| Source Files | 100 |
| Generated | 2026-04-28 |

## Nodes Breakdown

| Entity Type | Count |
|-------------|-------|
| `const` | 45 |
| `enum` | 43 |
| `function` | 605 |
| `module` | 34 |
| `struct` | 91 |
| `trait` | 1 |
| `type` | 10 |

## God Nodes

### Most Connected Files

| Node | Connections | Purpose |
|------|-------------|---------|
| `crates/omni-compiler/src/lib.rs` | 182 | bootstrap |
| `crates/omni-compiler/src/parser.rs` | 110 | bootstrap |
| `crates/omni-compiler/src/lsp.rs` | 95 | bootstrap |
| `crates/omni-compiler/src/diagnostics.rs` | 66 | bootstrap |
| `crates/omni-stdlib/src/lib.rs` | 64 | runtime |
| `crates/omni-compiler/src/async_effects.rs` | 59 | bootstrap |
| `crates/omni-compiler/tests/advanced_features.rs` | 57 | bootstrap |
| `crates/omni-compiler/src/levenshtein.rs` | 48 | bootstrap |
| `crates/omni-compiler/src/type_checker.rs` | 47 | bootstrap |
| `crates/codegen-mlir/src/lib.rs` | 44 | codegen |
| `crates/omni-compiler/src/type_export.rs` | 43 | bootstrap |
| `crates/omni-compiler/tests/diagnostic_ui.rs` | 38 | bootstrap |

### High-Centrality Entities

| Node | Kind | File | Connections |
|------|------|------|-------------|
| `push` | `function` | `crates/omni-stdlib/src/lib.rs:218` | 43 |
| `iter` | `function` | `crates/omni-compiler/src/async_effects.rs:338` | 38 |
| `clone` | `function` | `crates/omni-stdlib/src/lib.rs:49` | 34 |
| `tokenize` | `function` | `crates/omni-compiler/src/lexer.rs:118` | 29 |
| `contains` | `function` | `crates/omni-stdlib/src/lib.rs:111` | 25 |
| `Lexer` | `struct` | `crates/omni-compiler/src/lexer.rs:62` | 23 |
| `parse_program` | `function` | `crates/omni-compiler/src/parser.rs:131` | 23 |
| `join` | `function` | `crates/omni-compiler/src/async_effects.rs:96` | 23 |
| `example_module` | `function` | `crates/lir/src/lib.rs:87` | 21 |
| `Parser` | `struct` | `crates/omni-compiler/src/parser.rs:4` | 20 |
| `Expr` | `enum` | `crates/omni-compiler/src/ast.rs:9` | 20 |
| `Stmt` | `enum` | `crates/omni-compiler/src/ast.rs:71` | 17 |

## Communities

### Frontend

`crates/omni-compiler/src/lexer.rs`, `crates/omni-compiler/src/parser.rs`, `crates/omni-compiler/src/ast.rs`, `crates/omni-compiler/src/cst.rs`, `crates/omni-compiler/src/formatter.rs`, `crates/omni-compiler/src/diagnostics.rs`, `crates/omni-compiler/src/levenshtein.rs`

### Analysis

`crates/omni-compiler/src/resolver.rs`, `crates/omni-compiler/src/type_checker.rs`, `crates/omni-compiler/src/traits.rs`, `crates/omni-compiler/src/comptime.rs`, `crates/omni-compiler/src/async_effects.rs`, `crates/omni-compiler/src/lsp.rs`, `crates/omni-compiler/src/lsp_incr_db.rs`, `crates/omni-compiler/src/lsp_salsa_db.rs`

### Ir And Backend

`crates/omni-compiler/src/mir.rs`, `crates/omni-compiler/src/mir_optimize.rs`, `crates/omni-compiler/src/interpreter.rs`, `crates/omni-compiler/src/vm.rs`, `crates/omni-compiler/src/polonius.rs`, `crates/omni-compiler/src/codegen.rs`, `crates/omni-compiler/src/codegen_lir.rs`, `crates/omni-compiler/src/codegen_rust.rs`, `crates/codegen-llvm/src/lib.rs`, `crates/codegen-wasm/src/lib.rs`, `crates/codegen-mlir/src/lib.rs`, `crates/codegen-cranelift/src/lib.rs`

### Bootstrap And Distribution

`crates/omni-compiler/src/lib.rs`, `crates/omni-compiler/src/type_export.rs`, `crates/omni-compiler/src/abi_check.rs`, `crates/omni-compiler/build.rs`, `crates/omni-selfhost/src/bootstrap.rs`, `crates/omni-selfhost/src/lib.rs`, `crates/omni-selfhost/src/main.rs`, `crates/omni-stage0/src/main.rs`, `crates/omni-release/src/main.rs`, `crates/omni-stdlib/src/lib.rs`, `crates/polonius_engine_adapter/src/lib.rs`

### Tools And Tests

`scripts/test_fmt.rs`, `fuzz/fuzz_targets/lexer_parser.rs`, `fuzz/fuzz_targets/serialization.rs`, `crates/fuzz_harness/src/main.rs`, `crates/omni-fuzz/fuzz_targets/lexer_parser.rs`, `crates/omni-fuzz/fuzz_targets/serialization.rs`

## Critical Paths

- `lib.rs` -> `lexer.rs` -> `cst.rs` -> `parser.rs` -> `resolver.rs` -> `type_checker.rs` -> `mir.rs`
- `lib.rs` -> `type_export.rs` -> `abi_check.rs` -> `omni-stage0` CLI exports
- `polonius.rs` -> `polonius_engine_adapter` -> `omni-stage0 check-polonius`
- `bootstrap.rs` -> `omni-stage0` -> `omni-selfhost` pipeline
- `lsp.rs` -> `lsp_incr_db.rs` -> `parser.rs` / `type_checker.rs`

## Notes

- This graph was regenerated from the current workspace Rust sources.
- It includes core compiler code, bootstrap/runtime crates, codegen crates, fuzz targets, and workspace tools/tests.
- The earlier small snapshot has been superseded by this workspace-wide scan.
