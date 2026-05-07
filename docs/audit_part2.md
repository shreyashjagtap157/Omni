# Omni Codebase Exhaustive Audit — Part 2: Auxiliary Systems & Summary

**Date:** 2026-05-07 | **Continues from:** [Part 1](file:///C:/Users/ssjag/.gemini/antigravity/brain/d909f829-498b-4684-a0a9-e90f7ab10c43/audit_part1.md)

---

## 12. Effect System — `async_effects.rs` (317 lines)

### Spec Requirement (§6)
Full algebraic effect system: built-in effects (io, async, throw, panic, alloc, rand, time, log, pure), user-defined effects, effect handlers, effect inference, effect polymorphism.

| Feature | Status | Notes |
|---|---|---|
| `EffectKind` enum | ✅ Implemented | IO, Async, Throw, Panic, Alloc, Rand, Time, Log, Pure, Custom |
| `EffectSet` | ✅ Implemented | BTreeSet-based, with merge/subset/is_pure |
| `EffectHandler` struct | ✅ Implemented | Name + operations + handler body |
| `EffectCheckerCtx` | ✅ Implemented | Tracks function effects, installed handlers, violations |
| `check_effect_usage()` | ✅ Implemented | Validates handlers installed for required effects |
| Effect inference | ❌ Not implemented | No call-graph effect propagation |
| Effect polymorphism | ❌ Not implemented | |
| User-defined effects | ⚠️ Data only | Struct exists, no parsing/checking integration |
| **Pipeline integration** | ❌ **NOT INTEGRATED** | Not called from any `lib.rs` pipeline function |

---

## 13. Trait System — `traits.rs` (339 lines)

### Spec Requirement (§4.6)
Trait definitions, trait upcasting, negative bounds, custom diagnostic attributes, async traits.

| Feature | Status | Notes |
|---|---|---|
| `TraitDef` | ✅ Implemented | Name, methods, super traits, type params |
| `TraitImpl` | ✅ Implemented | Impl type, trait name, methods |
| `TraitRegistry` | ✅ Implemented | Register defs/impls, lookup, check satisfaction |
| `check_trait_bounds()` | ✅ Implemented | Verifies type implements required traits |
| Trait upcasting | ❌ Not implemented | |
| Negative bounds | ❌ Not implemented | |
| Custom diagnostic attributes | ❌ Not implemented | |
| Async traits | ❌ Not implemented | |
| **Pipeline integration** | ❌ **NOT INTEGRATED** | Not called from any pipeline function |

---

## 14. Macro System — `macros.rs` (469 lines)

### Spec Requirement (§8.11)
Declarative macros (hygienic, pattern-based), procedural macros (sandboxed), comptime codegen.

| Feature | Status | Notes |
|---|---|---|
| `MacroRule` / `MacroDef` | ✅ Implemented | Pattern + body template |
| `MacroExpander` | ✅ Implemented | Declarative pattern matching + expansion |
| `ProcMacroDef` | ✅ Implemented | `ProcMacroKind`: Derive, Attribute, FunctionLike |
| `ProcMacroEngine` | ✅ Implemented | Registration and invocation stubs |
| Hygienic expansion | ⚠️ Partial | Variable renaming exists but minimal |
| Sandboxed proc macros | ❌ Not implemented | No sandboxing |
| **Pipeline integration** | ❌ **NOT INTEGRATED** | |

---

## 15. Comptime — `comptime.rs` (537 lines)

### Spec Requirement (§4.9)
Compile-time evaluation of pure functions, string ops, type reflection, budget annotations.

| Feature | Status | Notes |
|---|---|---|
| `ComptimeValue` | ✅ Implemented | Int, Str, Bool, Float, Array, Struct, Type, Void |
| `ComptimeEvaluator` | ✅ Implemented | Evaluates expressions, handles calls |
| `evaluate_comptime_block()` | ✅ Implemented | Evaluates a block of comptime code |
| Comptime string operations | ✅ Implemented | concat, len, contains, starts_with, ends_with, replace |
| `TypeInfo` for reflection | ✅ Implemented | Name, fields, variants, methods |
| Budget system | ✅ Implemented | `ComptimeBudget` with ops limit |
| **Pipeline integration** | ❌ **NOT INTEGRATED** | |

---

## 16. Linear Types — `inout_desugar.rs` (213 lines) + `linear_types.rs`

### Spec Requirement (§5.5, §5.6)

| Feature | Status | Notes |
|---|---|---|
| Inout parameter desugaring | ✅ Implemented | Rewrites `inout` params to move-in/move-out |
| **Inout pipeline integration** | ✅ **INTEGRATED** | Called in `run_file()` and `check_file()` |
| `LinearTypeChecker` | ✅ Implemented | Tracks linear bindings, checks use-exactly-once |
| Linear type error reporting | ✅ Implemented | Unused, double-used, moved linear values |
| **Linear pipeline integration** | ❌ **NOT INTEGRATED** | |

---

## 17. Generational References — `generational_refs.rs` (304 lines)

### Spec Requirement (§5.4)

| Feature | Status | Notes |
|---|---|---|
| `Gen<T>` reference type | ✅ Implemented | Index + generation counter |
| `Arena<T>` allocator | ✅ Implemented | Alloc, get, get_mut, free, iteration |
| Generation validation | ✅ Implemented | O(1) check on dereference |
| **Pipeline integration** | ❌ **NOT INTEGRATED** | Standalone library, not part of type checking |

> [!NOTE]
> The `omni-stdlib` crate also has its own `Gen<T>` and `Arena<T>` implementation (324 lines). These are **duplicate** implementations that are not shared.

---

## 18. Security & Capabilities — `security.rs` (257 lines)

### Spec Requirement (§16)

| Feature | Status | Notes |
|---|---|---|
| `Capability` enum | ✅ Implemented | Filesystem, Network, Subprocess, Environment, FFI |
| `CapabilitySet` | ✅ Implemented | Grant, check, revoke |
| `FfiSandbox` | ✅ Implemented | Allow-list based |
| `CapabilityChecker` | ✅ Implemented | Validates capabilities against requirements |
| Effect-capability alignment | ❌ Not implemented | |
| Runtime capability tokens | ❌ Not implemented | |
| **Pipeline integration** | ❌ **NOT INTEGRATED** | |

---

## 19. Diagnostics — `diagnostics.rs` (253 lines)

### Spec Requirement (§14.4)

| Feature | Status | Notes |
|---|---|---|
| `DiagnosticCode` enum | ✅ Implemented | E1001-E7003 (70+ codes across 7 categories) |
| `Diagnostic` struct | ✅ Implemented | Code, message, span, severity, notes, suggestions |
| `DiagnosticEmitter` (text) | ✅ Implemented | Terminal-formatted output |
| JSON output | ✅ Implemented | `to_json()` serialization |
| Machine-applicable fixes | ⚠️ Partial | `Suggestion` struct exists, no automatic application |
| "Did you mean?" | ❌ Not implemented | No Levenshtein distance |
| **Pipeline integration** | ⚠️ Partial | Struct available but most errors use plain `String` |

---

## 20. Module System — `module_system.rs` (103 lines) + `omni_toml_parser.rs` (204 lines)

| Feature | Status | Notes |
|---|---|---|
| `omni.toml` parsing | ✅ Implemented | Name, version, edition, deps, modules, capabilities |
| Multi-file loading | ✅ Implemented | Loads modules from manifest |
| **Pipeline integration** | ✅ **INTEGRATED** | Used in `parse_file()` |
| Visibility enforcement | ❌ Not implemented | |
| PubGrub dependency resolution | ❌ Not implemented | |
| Lockfile (`omni.lock`) | ❌ Not implemented | |

---

## 21. Type Export & ABI — `type_export.rs` (458 lines) + `abi_check.rs` (213 lines)

| Feature | Status | Notes |
|---|---|---|
| Export to JSON | ✅ Implemented | Full struct/enum/fn signatures |
| Export to C headers | ✅ Implemented | typedef, struct, function prototypes |
| Export to Python | ✅ Implemented | ctypes/dataclass scaffolding |
| ABI compatibility checking | ✅ Implemented | Detects added/removed/changed items |
| **Pipeline integration** | ✅ **INTEGRATED** | Via `export_types_file()` and `check_abi_files()` |

---

## 22. LSP — `lsp.rs` (1318 lines)

| Feature | Status | Notes |
|---|---|---|
| LSP protocol server | ✅ Implemented | tower-lsp based |
| Diagnostics on save | ✅ Implemented | Runs check pipeline |
| Go-to-definition | ✅ Implemented | Symbol table lookup |
| Hover information | ✅ Implemented | Type info display |
| Completion | ✅ Implemented | Keyword + symbol completion |
| Semantic highlighting | ⚠️ Partial | Token-based, not semantic |
| Effect explorer | ❌ Not implemented | |
| Borrow checker visualization | ❌ Not implemented | |
| Incremental compilation | ⚠️ Stub | `lsp_incr_db.rs` (52 lines) exists but minimal |

---

## 23. External Crates

### `omni-stdlib` (324 lines)
- ✅ `Generation`, `Gen<T>`, `Arena<T>` — duplicate of `generational_refs.rs`
- Runtime support library, not linked to compiler pipeline

### `omni-stage0` (327 lines)
- ✅ Full CLI: parse, lex, parse-cst, fmt-cst, fmt, run, check, emit-mir, check-mir, run-mir, run-native, emit-wasm, export-types, bindgen, check-abi
- ✅ **INTEGRATED** — calls all `lib.rs` pipeline functions

### `omni-selfhost` (51 lines)
- ✅ Bootstrap pipeline driver: verify + self-host commands
- ⚠️ Depends on bootstrap module that likely stubs actual self-compilation

---

## MASTER SUMMARY

### Fully Implemented & Integrated ✅

| Component | Spec Section | Lines |
|---|---|---|
| Lexer (INDENT/DEDENT, all tokens) | §8, §12.1 | 896 |
| Parser (recursive descent + Pratt) | §12.1 | 2043 |
| AST definitions | §12.2 | 216 |
| CST + Formatter | §12.2, §14.2 | 698 |
| Name resolver (basic) | §12.1 | 265 |
| Type checker (basic unification) | §4, §12.1 | 1687 |
| MIR lowering | §12.2 | 927 |
| MIR optimization | §12.2 | 395 |
| Borrow checker (custom, not Polonius) | §5.2 | 818 |
| Codegen selector | §12.4 | 24 |
| MIR→LIR lowering | §12.2 | 270 |
| MIR→Rust codegen (fallback) | — | 209 |
| LIR definition | §12.2 | 98 |
| Cranelift backend (interpreter) | §12.4 | 934 |
| LLVM backend (C transpile) | §12.4 | 850 |
| WASM backend (real) | §17.3 | 337 |
| MLIR backend (textual) | §12.4 | 501 |
| AST interpreter | §21.3 | 1283 |
| MIR VM | — | 314 |
| Module system + omni.toml | §9 | 307 |
| Type export (JSON/C/Python) | §17 | 458 |
| ABI compatibility checker | §17.4 | 213 |
| Inout desugaring | §5.6 | 213 |
| Stage0 CLI | §14.1 | 327 |
| LSP server | §14.3 | 1318 |

### Implemented But NOT Integrated ⚠️

| Component | Spec Section | Lines | Issue |
|---|---|---|---|
| Effect system (`async_effects.rs`) | §6 | 317 | Not called from pipeline |
| Trait system (`traits.rs`) | §4.6 | 339 | Not called from pipeline |
| Macro system (`macros.rs`) | §8.11 | 469 | Not called from pipeline |
| Comptime evaluator (`comptime.rs`) | §4.9 | 537 | Not called from pipeline |
| Linear type checker | §5.5 | 245 | Not called from pipeline |
| Generational refs | §5.4 | 304 | Not called from pipeline |
| Security/capabilities (`security.rs`) | §16 | 257 | Not called from pipeline |
| Diagnostics infrastructure | §14.4 | 253 | Struct exists but errors use plain strings |

### Not Implemented ❌

| Feature | Spec Section | Priority |
|---|---|---|
| Effect inference & propagation | §6.3 | Phase 8 |
| Effect polymorphism | §6.7 | Phase 8 |
| User-defined effects + handlers | §6.4 | Phase 8 |
| Bidirectional type inference | §4.2 | Phase 2 |
| Exhaustive pattern checking | §4.7 | Phase 7 |
| Async/await execution | §6.5, §7 | Phase 8 |
| Structured concurrency | §7.2 | Phase 9 |
| Polonius engine integration | §12.3 | Phase 3 |
| Field projection in borrow checker | §5.3 | Phase 3 |
| Actual Cranelift JIT | §12.4 | Phase 6 |
| Actual LLVM/inkwell codegen | §12.4 | Phase 6 |
| Incremental compilation (Salsa) | §12.5 | Phase 6 |
| Parallel front-end | §12.6 | Phase 6 |
| Parser error recovery | §12.1 | Phase 1 |
| Variadic generics | §4.5 | Phase 7 |
| Trait upcasting/negative bounds | §4.6 | Phase 7 |
| PubGrub dependency resolver | §9.5 | Phase 4 |
| Comptime build scripts | §9.6 | Phase 4 |
| Test framework (@test, @effect_test) | §15.1 | Phase 6 |
| Contract annotations | §15.3 | Phase 7 |
| Replay debugging | §14.5 | Phase 9 |
| Fearless FFI sandboxing (real) | §16.3, §17.1 | Phase 10 |
| "Did you mean?" suggestions | §14.4 | Phase 1 |
| Async closures | §8.9 | Phase 8 |
| Let-chains | §4.7 | Phase 7 |
| Deconstructing parameters | §4.7 | Phase 7 |

### Key Architectural Issues

1. **8 standalone modules with no pipeline integration** — `async_effects`, `traits`, `macros`, `comptime`, `linear_types`, `generational_refs`, `security`, and `diagnostics` are all declared in `lib.rs` but never called by any pipeline function.

2. **Backend naming mismatch** — "Cranelift" backend is actually a software interpreter; "LLVM" backend is actually a C transpiler using Clang. Only WASM is a genuine target backend.

3. **Duplicate implementations** — `Gen<T>`/`Arena<T>` exist in both `omni-compiler/src/generational_refs.rs` and `omni-stdlib/src/lib.rs`.

4. **Diagnostic system unused** — Rich `Diagnostic` struct with codes, spans, and suggestions exists but the actual compiler uses `String` errors throughout.

5. **DefId system trivial** — All DefIds are hardcoded to `0`, making name resolution unable to distinguish between different symbols.
