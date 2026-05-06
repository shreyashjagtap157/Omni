# Omni Compiler — Exhaustive Architectural Audit (Part 1 of 2)
## Core Frontend & Mid-Level IR

**Audit Date:** 2026-05-06 | **Spec Version:** v2.0 | **Method:** File-by-file analysis of every source file compared against `docs/Omni_Complete_Specification.md`

---

## 1. Codebase Structure Overview

### Root Directory (`d:\Project\Omni`)
| Item | Notes |
|---|---|
| `Cargo.toml` | Workspace manifest |
| `docs/Omni_Complete_Specification.md` | 1400-line v2.0 spec (the authority) |
| `examples/` | Sample `.omni` files |
| `fuzz/` | Fuzz harness inputs |
| **Stale artifacts** | `debug_lexer.rs`, `debug_direct.rs`, `debug_parse.rs`, `debug_tokens.rs`, `test_tokenize.rs`, `test_tokenize_temp.rs`, `temp_type_checker.rs`, `fix.py`, `fix_tests*.py`, `replace.py`, `check_errors*.txt`, `last_mir_output.txt`, `nul`, `graph.json` — **all should be cleaned up** |

### Crate Map (14 crates + 1 loose file)

| Crate | Purpose | Lines | Status |
|---|---|---|---|
| `omni-compiler` | Core compiler (41 source files, 34 test files) | ~600KB | **Primary — active** |
| `lir` | Low-level IR definitions | 98 | Minimal scaffold |
| `codegen-cranelift` | Cranelift JIT + stack interpreter | 934 | **Functional** |
| `codegen-llvm` | LLVM backend via C emission + clang | 850 | **Functional** |
| `codegen-wasm` | WebAssembly backend via `wasm_encoder` | 337 | **Functional** |
| `codegen-mlir` | MLIR text emission + JIT bridge | 501 | Scaffold + text emitter |
| `omni-stdlib` | Runtime stdlib (`Gen<T>`, `Arena<T>`) | 324 | Partial |
| `polonius_engine_adapter` | Bridge to `polonius-engine` crate | 1516 | **Functional** |
| `polonius_engine_mock` | Mock borrow checker for tests | — | Functional |
| `omni-selfhost` | Self-hosting bootstrap stubs | 9 | Stub only |
| `omni-stage0` | Stage-0 compiler skeleton | — | Stub |
| `omni-release` | Release packaging | — | Stub |
| `omni-fuzz` | Fuzz harness crate | — | Stub |
| `fuzz_harness` | Additional fuzz support | — | Stub |
| **`resolver.absolute`** | **ScopeTree resolver** (loose file, not a crate) | 306 | **Written but NOT integrated** |

---

## 2. Core Compiler (`omni-compiler/src/`) — File-by-File

### 2.1 `lib.rs` (249 lines) — Driver/Orchestrator

**What it does:** Central pipeline entry point. Exposes `parse_file()`, `compile_file()`, `run_file()`, and `compile_and_run_file()`. Orchestrates: Source → Lexer → Parser → Type Checker → MIR Lowering → LIR Lowering → Codegen.

**Spec alignment:**
- ✅ Pipeline structure matches §12.1 (Source → Token → AST → MIR → LIR → Codegen)
- ✅ Uses `complete_lexer::tokenize_complete()` (unified lexer)
- ✅ Conditional codegen backend selection (`use_llvm` feature flag)
- ⚠️ **Missing steps:** No Effect Resolution pass (§12.1 step 4), no Name Resolution pass (§12.1 step 5), no separate Type+Effect Checking pass
- ⚠️ **No module system integration** — `module_system.rs` exists but is not called from the driver
- ❌ **No CST construction** — jumps directly from tokens to AST, skipping CST (§12.2)
- ❌ **No incremental compilation** — no Salsa query integration in the main pipeline

---

### 2.2 `complete_lexer.rs` (800+ lines) — Unified Lexer

**What it does:** Full tokenizer with INDENT/DEDENT layout engine, string interpolation, all v2.0 keywords, operators, built-in type tokens, comments (line/block/doc).

**Spec alignment (§8, §12.1):**
- ✅ INDENT/DEDENT synthetic tokens (§8.1)
- ✅ Newline-first syntax (§8.2)
- ✅ All operator tokens: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`, `!`, `->`, `=>`, `..`, `...`, `?`
- ✅ String interpolation scaffolding (`InterpolatedString` token)
- ✅ All v2.0 keywords: `fn`, `pub`, `async`, `await`, `effect`, `yield`, `spawn`, `cap`, `friend`, `trait`, `impl`, `struct`, `class`, `type`, `let`, `mut`, `const`, `static`, `return`, `break`, `continue`, `loop`, `while`, `for`, `in`, `where`, `use`, `mod`, `import`, `export`, `from`, `as`, `self`, `Self`, `inout`, `linear`, `unsafe`, `enum`, `variant`, `match`, `if`, `then`, `else`
- ✅ Built-in type tokens: `int`, `i8`..`i64`, `uint`, `u8`..`u64`, `f32`, `f64`, `char`, `bool`, `string`, `void`
- ✅ Comments: `--` line, `--- ---` block, `///` doc (§8.6)
- ✅ Hex/Oct/Bin number literals, float literals, raw strings, byte strings
- ⚠️ `@[attribute]` syntax: `@` tokenized but attribute parsing not fully wired
- ⚠️ No parallel lexing across files (§12.6) — single-threaded

**Verdict: ✅ COMPLETE for Phase 1 requirements. This is the most mature module.**

---

### 2.3 `lexer.rs` (19,964 bytes) — LEGACY Lexer

**What it does:** Old lexer with its own `Token`/`TokenKind` types. Still physically present in the source tree.

**Status:** 
- ❌ **DEPRECATED** — `complete_lexer.rs` is the canonical lexer
- ⚠️ `complete_parser.rs` (145 lines) still imports `lexer::Token` / `lexer::TokenKind` — **compilation-breaking dependency**
- All other modules have been migrated to `complete_lexer`

**Action needed:** Delete `lexer.rs` and fix/delete `complete_parser.rs` which depends on it.

---

### 2.4 `parser.rs` (1,959 lines) — Main Parser

**What it does:** Recursive descent + Pratt precedence parser. Produces `ast::Program` from `complete_lexer::Token` stream.

**Spec alignment (§8, §12.1):**
- ✅ Recursive descent with Pratt precedence (§12.1 — "Recursive Descent + Pratt")
- ✅ Precedence levels: `OrOr`, `AndAnd`, `EqEq`, `Lt`, `Plus`, `Star`
- ✅ Parses: `fn` (with `pub`, `async`, type params, effects, return type), `let`, `if/else`, `for`, `while`, `loop`, `return`, `break`, `continue`, `match` with guards, `struct`, `enum`, `impl`, `trait`, `type` aliases, `use`, `unsafe`, `linear`, `error set`, `spawn`, `channel`, `actor`, `tensor`, `simd`, `capability`, `ffi_sandbox`, `gc_mode`, `cancel_token`, `effect_handler`, `doc_comment`, `debug_session`
- ✅ Expressions: literals, variables, calls, binary ops, unary ops, field access, if-expr, interpolated strings, blocks, tuples, index, match-expr, range
- ✅ Pattern matching: wildcard, literal, variable, struct destructure, or-patterns
- ⚠️ **No panic-mode error recovery** — errors abort parsing immediately (§12.1 requires "panic-mode recovery")
- ⚠️ **No parallel multi-file parsing** (§12.6)
- ❌ **Missing:** Deconstructing function parameters (§4.7), let-chains (§4.7), async closures (§8.9), `comptime` blocks (§4.9)
- ❌ **No CST production** — parser produces AST directly, no lossless CST step (§12.2)

---

### 2.5 `ast.rs` (216 lines) — AST Definitions

**What it does:** Defines `Program`, `Stmt`, `Expr`, `Pattern`, `MatchArm`, `EnumVariant`, `InterpolatedFragment`.

**Stmt variants (26 total):**
`Print`, `Let`, `Fn` (with `is_public`, `is_async`, `type_params`, `params`, `ret_type`, `effects`, `body`), `ExprStmt`, `Block`, `If`, `Loop`, `For`, `While`, `Return`, `Break`, `Continue`, `Assign`, `ExprFieldAssign`, `WhileIn`, `Unsafe`, `LetLinear`, `Struct` (with `is_linear`), `ErrorSet`, `Impl`, `Trait`, `TypeAlias`, `Use`, `GcMode`, `CancelToken`, `EffectHandler`, `Spawn`, `Channel`, `Actor`, `WorkStealingExecutor`, `DeterministicRuntime`, `Tensor`, `Simd`, `DocComment`, `DebugSession`, `Capability`, `FfiSandbox`, `Enum` (with `is_sealed`)

**Expr variants (16):**
`StringLit`, `Number`, `Bool`, `Var`, `Call`, `BinaryOp`, `UnaryOp`, `FieldAccess`, `IfExpr`, `Interpolated`, `Block`, `Tuple`, `Index`, `Match`, `Range`

**Spec gaps:**
- ❌ No `Expr::Lambda`/`Expr::Closure` — closures not represented
- ❌ No `Expr::Await` — async expressions not representable
- ❌ No `Expr::Try` (`?` operator) — error propagation not in AST
- ❌ No `Expr::StructLit` — struct instantiation expressions missing
- ❌ No `Expr::EnumVariantLit` — enum variant construction missing
- ❌ No `Expr::Comptime` — compile-time evaluation blocks
- ❌ No `Stmt::Const` / `Stmt::Static` — despite keywords existing in lexer
- ❌ No `Stmt::Async` blocks
- ⚠️ `Fn` params are `Vec<String>` (names only) — **no type annotations on parameters**
- ⚠️ No span/location information on AST nodes — diagnostics cannot point to source

---

### 2.6 `cst.rs` (155 lines) — Concrete Syntax Tree

**What it does:** Defines `SyntaxKind`, `SyntaxToken`, `SyntaxNode`, `SyntaxElement` for a lossless tree. Used by the formatter.

**Spec alignment (§12.2):**
- ✅ Lossless representation preserving whitespace and comments
- ✅ Token kinds: `TokenIdent`, `TokenNumber`, `TokenString`, `TokenEquals`, `TokenOther`, `TokenNewline`, `TokenIndent`, `TokenDedent`, `TokenCommentLine`, `TokenDocComment`, `TokenCommentBlock`
- ✅ Node kinds: `Root`, `Statement`, `Block`, `Expression`, `Error`
- ⚠️ **Not Rowan-based** — spec §12.2 and Appendix B specify Rowan; this is a custom implementation
- ⚠️ **No CST↔AST lowering** — CST exists independently, not connected to the main pipeline

---

### 2.7 `diagnostics.rs` (253 lines) — Error Reporting

**What it does:** Defines `ErrorCode` enum (E1000–E7000), `DiagnosticLevel`, `Diagnostic` struct with span, help, notes, fix suggestions, secondary spans. Includes `emit_diagnostic()` for human-readable output.

**Spec alignment (§14.4):**
- ✅ Stable error codes (E1000–E7000 ranges)
- ✅ Primary span at source location
- ✅ Help notes with fix suggestions
- ✅ Secondary spans for related context
- ✅ Machine-applicable fix encoding (`FixSuggestion`)
- ⚠️ **No JSON output** (§14.4 requires JSON error output)
- ⚠️ **Not integrated into the pipeline** — most compiler passes use plain `String` errors, not `Diagnostic`
- ❌ **No diagnostic translations** (§14.4 — internationalization)
- ❌ **No custom diagnostic attributes for traits** (§4.6 — `@[diagnostic::on_unimplemented]`)

---

### 2.8 `resolver.rs` (265 lines) — Name Resolution (ACTIVE, LEGACY)

**What it does:** Stack-of-HashMaps name resolver. Resolves names in a single pass, walking scopes as a flat `Vec<HashMap>`.

**What it resolves:** `Let`, `Fn`, `Struct`, `Enum`, `Impl`, `Trait`, `TypeAlias`, `Use`, `ErrorSet`, `LetLinear`, `Actor`, `Capability`, `DocComment`

**Spec alignment (§12.1 step 5):**
- ⚠️ **Single-pass** — spec requires two-pass resolution
- ⚠️ **No DefId system** — uses flat symbol names, not unique DefIds
- ❌ **No ScopeTree** — uses `Vec<HashMap>` stack (O(n) parent lookup)
- ❌ **No undefined name errors with "Did you mean?" suggestions**
- ❌ **Not wired into the pipeline** — `lib.rs` does NOT call the resolver

### 2.8b `resolver.absolute` (306 lines) — ScopeTree Resolver (NOT INTEGRATED)

**What it does:** Complete ScopeTree-based resolver with `ScopeId`, `Scope`, `DefId`, `ScopeTree`, parent-chain lookup. Has `resolve_program()` that walks all statement types.

**Status:** 
- ✅ Architecturally correct: `ScopeId` + parent links, O(depth) lookup
- ✅ Handles all Stmt variants
- ✅ Has unit tests
- ❌ **It's a LOOSE FILE in `crates/`** — not a Rust module, not compiled, not imported anywhere
- ❌ Contains compilation errors (`fetch_add` without `Ordering` arg, function name casing violations)

---

### 2.9 `type_checker.rs` (1,687 lines) — Type System

**What it does:** Defines `Type` enum and `InferCtx` for bidirectional type inference with unification. Includes type checking for all Stmt/Expr variants.

**Type enum:** `Int`, `String`, `Bool`, `Unit`, `Never`, `Var(u32)` (inference variables), `Fn { params, ret, effects }`, `Struct { name, fields, is_linear }`, `Enum { name, variants, is_sealed }`, `Generic(String)`

**Spec alignment (§4):**
- ✅ Bidirectional type checking (§4.2) — `InferCtx` with `check`/`infer` modes
- ✅ Unification with occurs check
- ✅ Effect tracking in `Fn` types (u32 bitmask)
- ✅ Struct type with `is_linear` flag
- ✅ Enum type with `is_sealed` flag
- ✅ Generic type support (basic)
- ⚠️ **No `Option<T>` / `Result<T,E>` types** — spec §4.3/§4.4 core types missing
- ⚠️ **No trait bounds in type system** — `where T: Trait` not enforced during unification
- ⚠️ **Effects as u32 bitmask** — limits to 32 built-in effects, no user-defined effects
- ❌ **No type annotations on function parameters** — params are strings, not typed
- ❌ **No monomorphization** (§4.5)
- ❌ **No implied bounds** (§4.5)
- ❌ **No variadic generics** (§4.5)
- ❌ **No negative bounds** (§4.6)
- ❌ **No async trait support** (§4.6)
- ❌ **No `Try` trait** (§8.8)

---

### 2.10 `bidirectional_typer.rs` (410 lines) — Bidirectional Type Inference

**What it does:** Separate bidirectional type inference engine with `InferCtx`, effect set integration, unification. Checks expressions and statements with both `check` and `infer` modes.

**Status:** Functional but overlaps significantly with `type_checker.rs`. Both define inference contexts. **Architectural confusion — two type-checking modules.**

---

### 2.11 `effect_system.rs` (212 lines) — Built-in Effect Tracking

**What it does:** `EffectSet` using bitmask for built-in effects (`IO=0x01`, `ASYNC=0x02`, `PANIC=0x04`, `ALLOC=0x08`, `RAND=0x10`, `TIME=0x20`, `LOG=0x40`) plus `Vec<String>` for user-defined effects. Provides `union`, `is_subset`, `is_pure`, `format`.

**Spec alignment (§6):**
- ✅ All 8 built-in effect kinds present (§6.2)
- ✅ `pure` = empty set (§6.2)
- ✅ Effect polymorphism via `union` (§6.7)
- ⚠️ **No effect inference pass** — effects are not automatically propagated (§6.3)
- ⚠️ **No effect handler syntax/semantics** (§6.4)
- ❌ **No `throw<E>` as parameterized effect** — only `PANIC` bitmask
- ❌ **No user-defined effect definitions** (§6.4 — `effect Logging:`)
- ❌ **Not integrated into the type checker's function signatures**

### 2.11b `async_effects.rs` (317 lines) — Async Effect Types

**What it does:** Separate async/effect module with `FutureType`, `AsyncContext`, `Effect` enum, `EffectSet`, `StructuredConcurrencyScope`, `CancelToken`, `AsyncTransformResult`, `EffectHandler`.

**Status:** Contains richer `Effect` enum (Pure/Io/Async/Throw/Panic/Alloc/Rand/Time/Log/Custom) and structured concurrency types. **Overlaps with `effect_system.rs`** — two competing effect implementations.

---

### 2.12 `mir.rs` (927 lines) — Mid-Level IR

**What it does:** Defines `MirModule`, `MirFunction`, `BasicBlock`, `Instruction` (40+ variants). Includes `lower_program_to_mir()` for AST→MIR lowering.

**Instruction set:** `ConstInt`, `ConstStr`, `ConstBool`, `Move`, `LinearMove`, `BinOp`, `UnaryOp`, `Call`, `Print`, `Drop`, `LinearDrop`, `Jump`, `JumpIf`, `Label`, `Return`, `StructNew`, `FieldGet`, `FieldSet`, `VecNew`, `VecPush`, `VecGet`, `VecLen`, `BorrowShared`, `BorrowMut`, `BorrowRelease`, `Phi`, `Nop`, `EnumVariant`, `MatchBranch`, `StructDef`, `EnumDef`

**Spec alignment (§12.2):**
- ✅ CFG-based with basic blocks (§12.2)
- ✅ Explicit ownership: `Move`, `LinearMove`, `LinearDrop`
- ✅ Borrow tracking: `BorrowShared`, `BorrowMut`, `BorrowRelease`
- ✅ Drop insertion: `Drop` instruction
- ✅ Struct/enum operations
- ⚠️ **Lowering is partial** — only handles simple cases (literal assignments, basic binary ops, print, if/else, loops)
- ⚠️ **No field projection tracking** (§5.3)
- ❌ **Complex expressions not lowered** — nested calls, match, closures, tuples all skip MIR
- ❌ **No effect annotations on MIR** (§12.2 — "Effect-annotated AST" step missing)

### 2.12b `mir_optimize.rs` (15,876 bytes) — MIR Optimization

**What it does:** Dead code elimination, constant folding, copy propagation, unreachable block elimination on MIR.

**Status:** Functional optimizations. Well-tested. Aligns with §12.1 "MIR Optimization" step.

---

### 2.13 `polonius.rs` (37,708 bytes) — Borrow Checker

**What it does:** Polonius-based borrow checking on MIR. Extracts borrow facts, runs analysis, reports violations. Includes `BorrowFact`, `BorrowAnalysis`, fact export for the polonius engine adapter.

**Spec alignment (§5.2, §12.3):**
- ✅ Polonius algorithm (§12.3) — uses `polonius_engine_adapter`
- ✅ Shared/exclusive borrow tracking
- ✅ Use-after-move detection
- ✅ Conflicting borrow detection
- ✅ Fact export format compatible with polonius-engine
- ⚠️ **Field projections not tracked** (§5.3 — "different fields can be independently borrowed")
- ⚠️ **Linear type enforcement basic** — `LinearDrop` exists but no full "must be used exactly once" enforcement

---

### 2.14 `trait_system.rs` (255 lines) + `traits.rs` (10,515 bytes)

**What they do:** `trait_system.rs` defines `TraitSystem` with trait registration, method lookup, bound checking, negative bounds. `traits.rs` provides `TraitRegistry` with implementation checking and type compatibility.

**Spec alignment (§4.6):**
- ✅ Trait definitions with methods
- ✅ Trait bounds checking
- ✅ Negative bounds (`where T: !Copy`) — data structure exists
- ⚠️ **No trait upcasting** (§4.6)
- ⚠️ **No async trait methods** (§4.6)
- ⚠️ **No custom diagnostic attributes** (`@[diagnostic::on_unimplemented]`)
- ❌ **Not wired into the type checker** — trait bounds not enforced during type inference
- ❌ **Two overlapping modules** — `trait_system.rs` vs `traits.rs`

---

### 2.15 `linear_types.rs` (245 lines) — Linear Type Tracking

**What it does:** `LinearTypeChecker` tracks linear/affine/owned values. Verifies linear values are consumed exactly once per scope. Detects double-use and dropped-without-use violations.

**Spec alignment (§5.5):**
- ✅ Linear type modifier concept
- ✅ "Must be used exactly once" enforcement
- ✅ Double-use detection
- ⚠️ **Not integrated into the main pipeline** — standalone checker, not called from `lib.rs`

---

### 2.16 `generational_refs.rs` (304 lines) — Generational References

**What it does:** `Gen<T>`, `Arena<T>`, `Generation` counter. Arena allocation with generational validation. Use-after-free detection via generation mismatch.

**Spec alignment (§5.4):**
- ✅ `Gen<T>` with generation counter
- ✅ `Arena<T>` with alloc/free/get/get_mut
- ✅ O(1) use-after-free detection
- ✅ Free list for arena slot reuse
- ⚠️ Duplicated in `omni-stdlib` crate — same implementation exists there

---

*Part 2 covers: Interpreter, VM, Formatter, LSP, Codegen pipeline, External crates, Security, Module system, and the full Spec vs Implementation comparison matrix.*
