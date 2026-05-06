# Omni Compiler — Exhaustive Architectural Audit (Part 2 of 2)
## Backend, Tooling, External Crates & Spec Comparison Matrix

**Continues from Part 1** | **Audit Date:** 2026-05-06

---

## 3. Execution Backends

### 3.1 `interpreter.rs` (1,283 lines) — Tree-Walking Interpreter

**What it does:** Direct AST interpreter with `Value` enum (Int, Bool, Str, Vector, Struct, Enum, Closure, Result, Unit). Evaluates all Stmt/Expr variants. Rich built-in function library (80+ builtins: vector ops, string ops, math, IO, struct construction, pattern matching).

**Spec alignment (§19 Phase 2):**
- ✅ Satisfies "minimal interpreter for testing" requirement
- ✅ Hello world, fibonacci, fizzbuzz execute correctly
- ✅ Closures supported as `Value::Closure`
- ✅ Pattern matching with guards in `match`
- ⚠️ **No effect tracking during interpretation**
- ⚠️ **No borrow checking during interpretation**
- ⚠️ Built-ins are hardcoded strings — no stdlib trait dispatch

### 3.2 `vm.rs` (314 lines) — MIR Interpreter

**What it does:** Stack-based VM that executes MIR instructions directly. Handles `ConstInt/Str/Bool`, `Move`, `Print`, `Drop`, `Jump`, `JumpIf`, `BinOp`, `Call`, `Return`, `StructNew`, `FieldGet/Set`, `BorrowShared/Mut/Release`.

**Status:** Functional for the MIR subset that actually gets lowered. Provides a verification path for MIR correctness without needing codegen.

### 3.3 `codegen_lir.rs` (13,095 bytes) — MIR → LIR Lowering

**What it does:** Lowers `MirModule` to `lir::Module`. Maps MIR instructions to LIR stack operations (Const, Add, Sub, Mul, Div, Load, Store, Call, Ret, Jump, CondJump).

**Spec alignment (§12.1):**
- ✅ MIR → LIR lowering step exists
- ✅ Handles basic arithmetic, variables, control flow, function calls, print
- ⚠️ **Structs/enums not lowered to LIR** — only primitive operations
- ⚠️ **No ownership concepts stripped** — LIR should have "no ownership concepts" per §12.2

### 3.4 `codegen.rs` (24 lines) — Backend Router

**What it does:** Routes `compile_and_run(module)` to either Cranelift JIT (default) or LLVM backend based on `use_llvm` feature flag.

**Status:** ✅ Clean, correct routing logic.

---

## 4. External Codegen Crates

### 4.1 `codegen-cranelift` (934 lines)

**What it does:** Two execution paths:
1. **Stack interpreter** (`run_lir_interpreter`) — deterministic, dependency-free execution
2. **Cranelift JIT** (`compile_and_run_with_jit`) — real native code generation via `cranelift-codegen` + `cranelift-module` + `cranelift-jit`

**Status:** ✅ **Functional.** Both paths execute `lir::Module` and produce correct results. JIT generates x86-64 native code with proper function calls, including `host_print` for IO.

### 4.2 `codegen-llvm` (850 lines)

**What it does:** Emits C source code from LIR, compiles with `clang`, links, and runs the resulting binary. Also supports emitting standalone executables.

**Status:** ✅ **Functional.** Not actually using `inkwell` (LLVM bindings) — uses C emission as a bridge. Spec says LLVM via `inkwell` (Appendix B), but C emission is a valid intermediate step.

### 4.3 `codegen-wasm` (337 lines)

**What it does:** Emits WebAssembly binary using `wasm_encoder`. Proper type sections, function sections, export sections, import of `host_print`. Handles control flow with `block`/`loop`/`br_if`.

**Status:** ✅ **Functional.** Produces valid WASM modules that can run in wasmtime/Node.js.

### 4.4 `codegen-mlir` (501 lines)

**What it does:** Emits MLIR text format using func/arith/cf/memref/linalg dialects. Includes `TensorAddWorkload` for tensor operations. JIT execution bridges to Cranelift.

**Status:** ⚠️ Text emission works but actual MLIR compilation depends on external `mlir-opt` toolchain. JIT path falls back to Cranelift. Tensor workload is a fixture, not integrated.

### 4.5 `lir` Crate (98 lines)

**What it does:** Defines `Module`, `Function`, `Instr` (stack-based: Const, Add, Sub, Mul, Div, Load, Store, Call, Ret, Jump, CondJump, Drop, Nop), `Type` (I64, Void, Ptr).

**Spec gaps:**
- ❌ Only `I64`, `Void`, `Ptr` types — no floats, strings, structs, arrays
- ❌ No string operations at LIR level
- ❌ No struct/enum lowering at LIR level

---

## 5. Supporting Modules

### 5.1 `formatter.rs` (543 lines)

**What it does:** Two formatters: AST-based (`format_program`) and CST-based (`format_cst_source`). Handles all Stmt/Expr variants for pretty-printing.

**Spec alignment (§14.2):**
- ✅ CST-based formatting preserving comments
- ✅ Handles indentation, operators, string escaping
- ⚠️ **Not idempotent** — no property test verifying roundtrip
- ⚠️ **No `--check` mode** for CI
- ⚠️ **No import sorting** in strict mode

### 5.2 `lsp.rs` (47,013 bytes) — Language Server

**What it does:** Full LSP implementation using `tower-lsp`. Provides: diagnostics, go-to-definition, hover, completion, semantic tokens, document symbols, code actions, formatting, signature help, inlay hints, folding ranges.

**Spec alignment (§14.3):**
- ✅ LSP-compliant via `tower-lsp`
- ✅ Diagnostics publishing
- ✅ Go-to-definition
- ✅ Hover information
- ✅ Code completion
- ✅ Semantic tokens (type-based highlighting)
- ✅ Inlay hints
- ⚠️ **No effect explorer** (§14.3 — hover over calls to see effect set)
- ⚠️ **No borrow checker visualization** (§14.3)
- ⚠️ **Not query-based/incremental** — re-parses on every change (see `lsp_salsa_db.rs` for Salsa integration scaffold)

### 5.3 `lsp_salsa_db.rs` (176 lines) + `lsp_incr_db.rs` (1,585 bytes)

**What they do:** Salsa-backed incremental DB for LSP (behind `use_salsa_lsp` feature). Falls back to `SimpleLspDb` when Salsa is not enabled.

**Status:** Scaffold exists. Not the default path. §12.5 incremental compilation is aspirational.

### 5.4 `security.rs` (257 lines) — Capability System

**What it does:** `CapabilitySystem` with `Capability` enum (Io, Network, Filesystem, Environment, Random, Time, Process, Thread, Ffi), `CapabilityToken` with revocation, `FfiSandbox` with stack pointer and memory limit.

**Spec alignment (§16):**
- ✅ Capability types matching spec (§16.2)
- ✅ Token creation, checking, revocation
- ✅ FFI sandbox structure (§16.3)
- ⚠️ **Not enforced at compile time** — runtime-only checking
- ❌ **No capability-effect alignment** (§16.2 — capabilities and effects should be unified)
- ❌ **No package signing/verification** (§16.4)

### 5.5 `module_system.rs` (103 lines)

**What it does:** Simple module loader that reads `omni.toml` for module declarations and loads `.omni` files.

**Spec alignment (§9):**
- ⚠️ **Very basic** — just file loading, no visibility levels
- ❌ **No hierarchical modules** (§9.1)
- ❌ **No visibility enforcement** (§9.2 — `pub(mod)`, `pub(pkg)`, etc.)
- ❌ **No package manager/dependency resolution** (§9.5)
- ❌ **Not called from main pipeline**

### 5.6 `inout_desugar.rs` (213 lines)

**What it does:** Desugars `inout` parameters into move-in/move-out MIR semantics. Detects `inout_` prefixed params and rewrites to `LetLinear` + `LinearMove`.

**Spec alignment (§5.6):** ✅ Correct concept. ⚠️ Convention-based (`inout_` prefix) rather than keyword-based.

### 5.7 `comptime.rs` (19,325 bytes)

**What it does:** Compile-time evaluation engine. Evaluates `comptime` expressions, constant folding, type reflection stubs.

**Spec alignment (§4.9):** ⚠️ Basic constant folding works. ❌ No comptime string operations, no comptime type reflection, no budget annotations.

### 5.8 `macros.rs` (15,946 bytes)

**What it does:** Declarative macro system with pattern matching on token streams. Hygienic expansion.

**Spec alignment (§8.11):** ⚠️ Declarative macros present. ❌ No procedural macros (sandboxed). ❌ No comptime code generation.

### 5.9 `levenshtein.rs` (15,274 bytes)

**What it does:** Levenshtein edit distance for "Did you mean?" suggestions on undefined identifiers.

**Spec alignment (§14.4):** ✅ "Did you mean?" foundation exists. ⚠️ Not wired into error reporting in the main pipeline.

### 5.10 `abi_check.rs` (213 lines) + `type_export.rs` (16,503 bytes)

**What they do:** Export public API as structured document. Compare two export documents to detect breaking changes (parameter/return type changes, field changes, variant changes).

**Spec alignment (§17.4, §9.5):** ✅ API compatibility checking concept. ⚠️ Not integrated into a package manager.

### 5.11 `phase1_bridge.rs` (160 lines) + `complete_parser.rs` (145 lines)

**What they do:** Bridge modules mapping `complete_lexer` tokens to the parser. `phase1_bridge.rs` maps directly to `complete_lexer::TokenKind`. `complete_parser.rs` maps to the **legacy** `lexer::TokenKind`.

**Status:** `complete_parser.rs` has a **compilation-breaking dependency on `lexer.rs`** (the deprecated module). Must be deleted or rewritten.

---

## 6. External Crates

### 6.1 `omni-stdlib` (324 lines)
Duplicates `Gen<T>`, `Arena<T>` from `generational_refs.rs`. Also has `HashMap`, `String`, `Vec` stubs, `IoCapability`, `AsyncRuntime` scaffolds.

### 6.2 `polonius_engine_adapter` (1,516 lines)
Full adapter bridging Omni's borrow fact format to `polonius-engine`'s `AllFacts<T>`. Defines `AtomId`, `SimpleFacts`, parses textual fact exports, runs the engine. ✅ **Functional and well-tested.**

### 6.3 `omni-selfhost` (9 lines)
Stub with `build_stage1`, `build_stage2`, `compare_stages` imports. ❌ No actual self-hosting logic.

---

## 7. Spec vs Implementation — Comparison Matrix

### §4 Type System

| Feature | Spec Section | Status | Details |
|---|---|---|---|
| Static typing | §4.1 | ✅ | `type_checker.rs` |
| Effect annotations in types | §4.1 | ⚠️ Partial | u32 bitmask in `Fn` type, not parameterized |
| Bidirectional type checking | §4.2 | ✅ | Two modules (`type_checker.rs`, `bidirectional_typer.rs`) |
| Option\<T\> | §4.3 | ❌ | Not in type system |
| Result\<T,E\> | §4.4 | ❌ | Not in type system |
| Error set types | §4.4 | ⚠️ | AST node exists, no type-system representation |
| Monomorphization | §4.5 | ❌ | Not implemented |
| Implied bounds | §4.5 | ❌ | Not implemented |
| Variadic generics | §4.5 | ❌ | Not implemented |
| Trait upcasting | §4.6 | ❌ | Not implemented |
| Negative bounds | §4.6 | ⚠️ | Data structure exists, not enforced |
| Async traits | §4.6 | ❌ | Not implemented |
| Custom diagnostic attrs | §4.6 | ❌ | Not implemented |
| Exhaustive pattern matching | §4.7 | ⚠️ | Parser handles, no usefulness algorithm |
| Or-patterns | §4.7 | ✅ | In parser + AST |
| Let-chains | §4.7 | ❌ | Not parsed |
| Sealed enums | §4.8 | ⚠️ | AST flag exists, not enforced |
| Comptime evaluation | §4.9 | ⚠️ | Basic constant folding only |
| Compile-time reflection | §4.10 | ❌ | Not implemented |

### §5 Memory Model & Ownership

| Feature | Spec Section | Status | Details |
|---|---|---|---|
| Ownership tracking | §5.1 | ⚠️ | MIR `Move`/`Drop` but no enforcement in type checker |
| Shared/exclusive borrows | §5.2 | ✅ | MIR + Polonius |
| Polonius borrow checker | §5.2 | ✅ | `polonius.rs` + `polonius_engine_adapter` |
| Field projections | §5.3 | ❌ | Not tracked |
| Generational references | §5.4 | ✅ | `generational_refs.rs` + `omni-stdlib` |
| Linear types | §5.5 | ⚠️ | `linear_types.rs` exists, not integrated |
| Inout parameters | §5.6 | ⚠️ | `inout_desugar.rs` exists, convention-based |
| Arena allocation | §5.7 | ✅ | `Arena<T>` implemented |
| Safe/unsafe boundary | §5.8 | ⚠️ | AST `Unsafe` block, no compiler enforcement |
| GC compatibility | §5.9 | ⚠️ | AST `GcMode` node, no runtime |

### §6 Effect System

| Feature | Spec Section | Status | Details |
|---|---|---|---|
| Built-in effect kinds | §6.2 | ✅ | Both `effect_system.rs` and `async_effects.rs` |
| Effect inference | §6.3 | ❌ | Not implemented |
| Effect handlers | §6.4 | ❌ | AST node only |
| Async as effect | §6.5 | ⚠️ | `async_effects.rs` scaffold |
| Generators | §6.6 | ❌ | Not implemented |
| Effect polymorphism | §6.7 | ⚠️ | `union` in EffectSet, not in generics |

### §7 Concurrency

| Feature | Spec Section | Status | Details |
|---|---|---|---|
| Structured concurrency | §7.2 | ⚠️ | Data structures in `async_effects.rs`, no runtime |
| Explicit cancellation | §7.3 | ⚠️ | `CancelToken` AST + struct, no runtime |
| Actors | §7.5 | ⚠️ | AST node only |
| Channels | §7.4 | ⚠️ | AST node only |

### §8 Syntax

| Feature | Spec Section | Status | Details |
|---|---|---|---|
| Indentation blocks | §8.1 | ✅ | INDENT/DEDENT in lexer |
| Newline-first | §8.2 | ✅ | Lexer handles |
| Expression orientation | §8.3 | ⚠️ | `if`/`match` as expressions, no `block`/`try` |
| String interpolation | §8.10 | ✅ | Lexer + parser |
| Declarative macros | §8.11 | ⚠️ | Basic in `macros.rs` |
| Async closures | §8.9 | ❌ | Not parsed/represented |

### §9 Module System

| Feature | Spec Section | Status | Details |
|---|---|---|---|
| File modules | §9.1 | ⚠️ | `module_system.rs` basic loader |
| Visibility levels | §9.2 | ❌ | Not enforced |
| Import system | §9.3 | ⚠️ | `use` parsed, no resolution |
| omni.toml manifest | §9.4 | ⚠️ | Basic parser in `omni_toml_parser.rs` |
| PubGrub dependency resolution | §9.5 | ❌ | Not implemented |

### §12 Compilation Model

| Feature | Spec Section | Status | Details |
|---|---|---|---|
| Lexer → Token stream | §12.1 | ✅ | `complete_lexer.rs` |
| Parser → CST | §12.1 | ❌ | Parser produces AST directly |
| CST → AST lowering | §12.1 | ❌ | No lowering step |
| Effect resolution pass | §12.1 | ❌ | Not implemented as separate pass |
| Name resolution | §12.1 | ❌ | Not wired in (exists in code but not called) |
| Type inference | §12.1 | ✅ | `type_checker.rs` |
| MIR lowering | §12.1 | ⚠️ | Partial — simple cases only |
| Borrow checker | §12.1 | ✅ | Polonius-based |
| MIR optimization | §12.1 | ✅ | `mir_optimize.rs` |
| LIR lowering | §12.1 | ⚠️ | Basic primitives only |
| Codegen (Cranelift) | §12.4 | ✅ | JIT + interpreter |
| Codegen (LLVM) | §12.4 | ⚠️ | Via C emission, not `inkwell` |
| Codegen (WASM) | §12.4 | ✅ | `wasm_encoder` |
| Codegen (MLIR) | §12.4 | ⚠️ | Text emission only |
| Incremental compilation | §12.5 | ❌ | Salsa scaffold, not active |
| Parallel frontend | §12.6 | ❌ | Single-threaded |

### §14 Tooling

| Feature | Spec Section | Status | Details |
|---|---|---|---|
| CLI (`omni` commands) | §14.1 | ⚠️ | Basic `run`/`build`/`check` |
| Formatter | §14.2 | ⚠️ | AST + CST formatters, not idempotent |
| LSP | §14.3 | ✅ | Full LSP with many features |
| Diagnostic quality | §14.4 | ⚠️ | Error codes exist, not used throughout |
| Debugger (DAP) | §14.5 | ❌ | Not implemented |
| Doc generator | §14.6 | ❌ | Not implemented |

### §16 Security

| Feature | Spec Section | Status | Details |
|---|---|---|---|
| Capability system | §16.2 | ⚠️ | Runtime struct, not compile-time |
| FFI sandboxing | §16.3 | ⚠️ | Struct exists, no stack switching |
| Package security | §16.4 | ❌ | Not implemented |

---

## 8. Critical Issues Summary

### 🔴 Blocking Issues (Must Fix)

1. **`complete_parser.rs` imports deleted `lexer.rs`** — compilation error waiting to happen
2. **`resolver.absolute` is a loose file** — ScopeTree resolver not integrated as a module
3. **Name resolver not called from pipeline** — names are never resolved
4. **Two competing type-checking modules** — `type_checker.rs` vs `bidirectional_typer.rs`
5. **Two competing effect modules** — `effect_system.rs` vs `async_effects.rs`
6. **Two competing trait modules** — `trait_system.rs` vs `traits.rs`
7. **AST has no type annotations on function params** — `Vec<String>` instead of typed params
8. **AST has no spans** — diagnostics cannot point to source locations

### 🟡 Architectural Debt

9. **No CST → AST lowering** — parser skips CST entirely
10. **Legacy `lexer.rs` still in tree** — 20KB of dead code
11. **Root directory cluttered** — debug scripts, .pdb, .o files, temp files
12. **Duplicated `Gen<T>/Arena<T>`** — in both `generational_refs.rs` and `omni-stdlib`
13. **`phase1_bridge.rs` is redundant** — identity mapping since parser now uses `complete_lexer` directly
14. **Linear types, traits, effects, capabilities all exist but none are integrated into the compilation pipeline**

### 🟢 Working End-to-End

15. **Lexer → Parser → Interpreter path** works for basic programs
16. **Lexer → Parser → Type Check → MIR → LIR → Cranelift JIT** works for arithmetic + print
17. **Lexer → Parser → Type Check → MIR → LIR → WASM** works for arithmetic
18. **Polonius borrow checking on MIR** works for basic borrow scenarios
19. **LSP provides diagnostics, completion, go-to-def** for basic editing

---

## 9. Recommended Priority Order

Based on the spec's own §21.3 "Recommended Immediate Focus":

1. **Delete dead code:** `lexer.rs`, `complete_parser.rs`, `phase1_bridge.rs`, root debug files
2. **Integrate ScopeTree resolver:** Move `resolver.absolute` → `resolver.rs`, wire into `lib.rs`
3. **Unify overlapping modules:** Pick one of each pair (type_checker, effect_system, trait_system)
4. **Add spans to AST:** Every node needs source location
5. **Add typed params to `Fn`:** `Vec<(String, TypeAnnotation)>` instead of `Vec<String>`
6. **Wire diagnostics system:** Replace `String` errors with `Diagnostic` throughout
7. **Complete MIR lowering:** Handle all AST node types
8. **Validate end-to-end:** `omni build hello.omni` → binary → prints "Hello, World!"
