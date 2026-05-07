# Omni Codebase Exhaustive Audit — Part 1: Core Compiler Pipeline

**Date:** 2026-05-07 | **Method:** File-by-file source analysis vs `docs/Omni_Complete_Specification.md`

---

## Codebase Structure Map

```
d:\Project\Omni\
├── Cargo.toml                          # Workspace root
├── docs/
│   └── Omni_Complete_Specification.md  # 1400-line v2.0 specification
├── crates/
│   ├── omni-compiler/src/              # Core compiler (29 source files)
│   │   ├── lib.rs                      # Pipeline driver + module declarations
│   │   ├── ast.rs                      # AST data structures
│   │   ├── complete_lexer.rs           # Full-spec lexer (896 lines)
│   │   ├── parser.rs                   # Recursive descent + Pratt parser (2043 lines)
│   │   ├── cst.rs                      # Concrete Syntax Tree (155 lines)
│   │   ├── formatter.rs               # CST-based formatter (543 lines)
│   │   ├── resolver.rs                # Name resolution (265 lines)
│   │   ├── type_checker.rs            # Type checker + inference (1687 lines)
│   │   ├── mir.rs                      # MIR definition + AST→MIR lowering (927 lines)
│   │   ├── mir_optimize.rs            # MIR optimizations (395 lines)
│   │   ├── polonius.rs                # Borrow checker (818 lines)
│   │   ├── codegen.rs                 # Backend selector (24 lines)
│   │   ├── codegen_lir.rs            # MIR→LIR lowering (270 lines)
│   │   ├── codegen_rust.rs           # MIR→Rust codegen (209 lines)
│   │   ├── interpreter.rs            # AST tree-walk interpreter (1283 lines)
│   │   ├── vm.rs                      # MIR VM interpreter (314 lines)
│   │   ├── async_effects.rs          # Effect + async types (317 lines)
│   │   ├── comptime.rs               # Compile-time evaluator (537 lines)
│   │   ├── diagnostics.rs            # Diagnostic infrastructure (253 lines)
│   │   ├── traits.rs                  # Trait system (339 lines)
│   │   ├── macros.rs                  # Macro system (469 lines)
│   │   ├── linear_types.rs           # Linear type checker (245 lines)
│   │   ├── generational_refs.rs      # Gen<T> + Arena<T> (304 lines)
│   │   ├── inout_desugar.rs          # Inout parameter desugaring (213 lines)
│   │   ├── security.rs               # Capability system + FFI sandbox (257 lines)
│   │   ├── integration.rs            # OmniInterpreter facade (142 lines)
│   │   ├── module_system.rs          # Module loading (103 lines)
│   │   ├── omni_toml_parser.rs       # Manifest parser (204 lines)
│   │   ├── type_export.rs            # Type export/bindgen (458 lines)
│   │   ├── abi_check.rs              # ABI compatibility checker (213 lines)
│   │   ├── lsp.rs                    # Language server (1318 lines)
│   │   ├── lsp_incr_db.rs           # Simple incremental DB (52 lines)
│   │   ├── lsp_salsa_db.rs          # Salsa-backed DB (176 lines)
│   │   ├── llvm_detect.rs           # LLVM detection (69 lines)
│   │   └── tests/                    # Test files
│   ├── lir/src/lib.rs                # LIR IR definition (98 lines)
│   ├── codegen-cranelift/src/lib.rs  # Cranelift backend (934 lines)
│   ├── codegen-llvm/src/lib.rs       # LLVM backend (850 lines)
│   ├── codegen-wasm/src/lib.rs       # WASM backend (337 lines)
│   ├── codegen-mlir/src/lib.rs       # MLIR backend (501 lines)
│   ├── omni-stdlib/src/lib.rs        # Stdlib Rust runtime (324 lines)
│   ├── omni-stage0/src/main.rs       # Stage0 CLI (327 lines)
│   └── omni-selfhost/src/            # Self-hosting pipeline
├── omni/stdlib/                       # Omni source stdlib files
├── examples/                          # Example .omni files
├── scripts/                          # Build/CI scripts
└── fuzz/                             # Fuzz targets
```

---

## 1. Driver / Pipeline — `lib.rs` (253 lines)

### Spec Requirement (§12.1)
The full pipeline should be: Source → Lexer → CST → AST → Effect Resolution → Name Resolution → Type Inference → Type+Effect Checking → MIR Lowering → Borrow Checker (Polonius) → MIR Optimization → LIR Lowering → Codegen → Linker.

### Current Implementation

| Pipeline Function | Passes Executed | Status |
|---|---|---|
| `run_file()` | parse → inout_desugar → resolver → type_checker → interpreter | ✅ Functional |
| `check_file()` | parse → inout_desugar → resolver → type_checker | ✅ Functional |
| `emit_mir_file()` | parse → resolver → MIR lowering → format | ✅ Functional |
| `emit_lir_file()` | parse → MIR → MIR optimize → LIR lowering | ✅ Functional |
| `check_mir_file()` | parse → resolver → MIR lowering → polonius check | ✅ Functional |
| `run_mir_vm_file()` | parse → MIR lowering → VM | ✅ Functional |
| `run_native_file()` | parse → resolver → MIR → optimize → LIR → codegen | ✅ Functional |
| `emit_wasm_file()` | parse → MIR → optimize → LIR → WASM emit | ✅ Functional |
| `format_file()` | lex → CST → format → write | ✅ Functional |
| `export_types_file()` | parse → export (JSON/C/Python) | ✅ Functional |
| `check_abi_files()` | parse both → export → compare | ✅ Functional |

### What's Missing from Spec Pipeline
- ❌ **Effect Resolution pass** — No separate effect resolution pass before name resolution. Effects are defined in `async_effects.rs` but not integrated into the AST pipeline.
- ❌ **Bidirectional type inference** — `type_checker.rs` has HM-style unification, not full bidirectional.
- ❌ **Type+Effect Checking as unified pass** — Effects are a bitfield (`u8`) in the type checker, not a structured effect set.
- ❌ **Parallel front-end** — No parallel file parsing.
- ❌ **Incremental compilation** — Salsa DB exists but is not wired into the main pipeline.

### Integration Status
- ✅ Module system (`omni.toml` + multi-file loading) is integrated into `parse_file()`.
- ✅ Stdlib auto-loading works for `omni/stdlib/core.omni` and `collections.omni`.
- ✅ Inout desugaring is integrated into `run_file()` and `check_file()`.
- ⚠️ `linear_types.rs`, `traits.rs`, `macros.rs`, `comptime.rs`, `security.rs` — all **declared as modules** in `lib.rs` but **NOT called** by any pipeline function. They are standalone libraries with no pipeline integration.

---

## 2. Lexer — `complete_lexer.rs` (896 lines)

### Spec Requirement (§8, §12.1)
Full token set, INDENT/DEDENT layout engine, string interpolation, all keywords, all operators, comments (line, block, doc), raw strings, byte strings, hex/oct/bin numbers.

### Implementation Status

| Feature | Status | Notes |
|---|---|---|
| Full keyword set (40+ keywords) | ✅ Implemented | All spec keywords present in `TokenKind` enum |
| Built-in type keywords | ✅ Implemented | Int, Int8..64, UInt, UInt8..64, Float32/64, Char, Bool, String, Void |
| INDENT/DEDENT layout engine | ✅ Implemented | Stack-based, `at_line_start` tracking |
| Nesting-aware indentation | ✅ Implemented | `nesting` counter for parens/brackets/braces |
| String literals | ✅ Implemented | Escape sequences handled |
| Interpolated strings | ✅ Implemented | `f"..."` with `{expr}` interpolation |
| Raw strings | ✅ Implemented | `r#"..."#` prefix (fixed in previous session) |
| Byte strings | ✅ Implemented | `b"..."` prefix (fixed in previous session) |
| Number literals | ✅ Implemented | Integer, float, hex (`0x`), octal (`0o`), binary (`0b`) |
| Operators | ✅ Implemented | All standard operators including `..`, `...`, `::`, `=>`, `->` |
| Line comments (`--`) | ✅ Implemented | |
| Block comments (`--- ... ---`) | ✅ Implemented | |
| Doc comments (`///`) | ✅ Implemented | |
| Error recovery | ❌ Not implemented | Lexer returns first error, no recovery |
| Span/position tracking | ⚠️ Partial | Line+col tracked per token, but no byte-offset spans |
| UTF-8 identifiers | ❌ Not implemented | ASCII only |

### Known Issues (Fixed in Previous Session)
- ✅ Raw string `r#` prefix no longer greedily consumes `return`, `break`, etc.
- ✅ Byte string `b"` prefix no longer greedily consumes `break`.
- ✅ Stale cursor bug after indentation processing fixed.

---

## 3. Parser — `parser.rs` (2043 lines)

### Spec Requirement (§12.1, §4, §8)
Recursive descent + Pratt for expressions, panic-mode error recovery, all statement and expression types from the spec.

### Implementation Status

| Feature | Status | Notes |
|---|---|---|
| Pratt precedence parsing | ✅ Implemented | 7 precedence levels (OrOr through Star) |
| `let` bindings | ✅ Implemented | `Let(name, expr)` |
| `let linear` bindings | ✅ Implemented | `LetLinear(name, expr)` |
| Function definitions | ✅ Implemented | `Fn { name, is_public, is_async, type_params, params, ret_type, effects, body }` |
| Struct definitions | ✅ Implemented | `Struct { name, fields, is_linear }` |
| Enum definitions | ✅ Implemented | `Enum { name, variants, is_sealed }` |
| Error set definitions | ✅ Implemented | `ErrorSet { name, variants }` |
| If/else | ✅ Implemented | With bindings support |
| Loop/while/for | ✅ Implemented | Including `WhileIn` |
| Match expressions | ✅ Implemented | With patterns: Wildcard, Literal, Var, Struct, Or |
| Return/break/continue | ✅ Implemented | |
| Trait definitions | ✅ Implemented | `Trait { name, type_params, methods }` |
| Impl blocks | ✅ Implemented | `Impl { target, type_params, methods }` |
| Type aliases | ✅ Implemented | `TypeAlias { name, type_params, target }` |
| Use/import | ✅ Implemented | `Use { path, alias }` |
| Unsafe blocks | ✅ Implemented | `Unsafe { body }` |
| Field access expressions | ✅ Implemented | `FieldAccess { base, field }` |
| Index expressions | ✅ Implemented | `Index(base, index)` |
| Tuple expressions | ✅ Implemented | `Tuple(Vec<Expr>)` |
| Range expressions | ✅ Implemented | `Range { start, end, inclusive }` |
| Block expressions | ✅ Implemented | `Block(Vec<Stmt>)` |
| String interpolation | ✅ Implemented | `Interpolated(Vec<InterpolatedFragment>)` |
| If expressions | ✅ Implemented | `IfExpr { cond, then, else_ }` |
| Binary/unary ops | ✅ Implemented | All arithmetic, comparison, logical |
| Function calls | ✅ Implemented | `Call(name, args)` |
| Method calls | ⚠️ Partial | Parsed as `FieldAccess` + separate call; no method resolution |
| Async/await | ⚠️ Stub in AST | AST has `is_async` on `Fn`, no `await` expression node |
| Effect annotations | ⚠️ Stub | Parsed as `Vec<String>` on `Fn`, no structured effect type |
| Panic-mode recovery | ❌ Not implemented | Parser returns first error |
| Let-chains (`if let ... and let ...`) | ❌ Not implemented | |
| Deconstructing parameters | ❌ Not implemented | |
| Async closures | ❌ Not implemented | |
| Curly brace blocks | ✅ Implemented | Added in previous session |

### AST Nodes Present But Not Lowered Through Pipeline
These AST nodes exist and are parsed, but have no downstream processing:

| AST Node | Parsed? | Resolved? | Type-checked? | MIR Lowered? |
|---|---|---|---|---|
| `Stmt::GcMode` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::CancelToken` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::EffectHandler` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::Spawn` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::Channel` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::Actor` | ✅ | ✅ (registers name) | ✅ (no-op) | ❌ |
| `Stmt::WorkStealingExecutor` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::DeterministicRuntime` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::Tensor` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::Simd` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::DebugSession` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::Capability` | ✅ | ✅ (registers name) | ✅ (no-op) | ❌ |
| `Stmt::FfiSandbox` | ✅ | ✅ (no-op) | ✅ (no-op) | ❌ |
| `Stmt::DocComment` | ✅ | ✅ (registers target) | ✅ (no-op) | ❌ |

---

## 4. Name Resolver — `resolver.rs` (265 lines)

### Spec Requirement (§12.1, Phase 2)
Two-pass name resolution, scope tree, DefId system, use declarations, implied bounds.

### Implementation Status

| Feature | Status | Notes |
|---|---|---|
| Scope stack | ✅ Implemented | `Vec<HashMap<String, DefId>>` |
| Function name pre-registration | ✅ Implemented | First pass registers all top-level `Fn` names |
| Variable resolution | ✅ Implemented | Walks scope stack in reverse |
| Function call resolution | ✅ Implemented | Checks function name in scope |
| Nested scope handling | ✅ Implemented | Block, if, loop, for, while, unsafe create scopes |
| Struct/enum/trait/impl registration | ✅ Implemented | Names registered in current scope |
| Use declarations | ✅ Implemented | Path-based with optional alias |
| Error reporting | ✅ Implemented | "Undefined name" and "Undefined function" errors |
| DefId system | ⚠️ Trivial | All DefIds are `0` — no unique identifier assignment |
| Two-pass resolution | ⚠️ Partial | Pre-registers top-level fns, but no full two-pass |
| Scope tree construction | ❌ Not implemented | No persistent scope tree returned |
| Implied bounds | ❌ Not implemented | |
| Deep expression resolution | ⚠️ Partial | Only resolves `Expr::Var` and `Expr::Call` at top level of let/print/assign; does not recurse into nested BinaryOp, FieldAccess, etc. |

---

## 5. Type Checker — `type_checker.rs` (1687 lines)

### Spec Requirement (§4, §12.1, Phase 2)
Bidirectional type inference, unification, effect inference, trait bound checking, linear type tracking.

### Implementation Status

| Feature | Status | Notes |
|---|---|---|
| Type representation | ✅ Implemented | Int, String, Bool, Var, Generic, Fn, Struct, Enum, Unit, Never |
| Type inference variables | ✅ Implemented | `InferCtx` with fresh vars and substitution |
| Unification | ✅ Implemented | With occurs check, recursive resolution |
| Function type checking | ✅ Implemented | Param types, return type, effect flags |
| Struct type checking | ✅ Implemented | Field types, linear flag |
| Enum type checking | ✅ Implemented | Variant types, sealed flag |
| Effect tracking | ⚠️ Minimal | `u8` bitfield: `EF_IO`, `EF_PURE`, `EF_ASYNC`, `EF_PANIC` — 4 effects only |
| Trait definitions | ⚠️ Data only | `Trait` struct defined with bounds, but no trait resolution/checking |
| Linear type state tracking | ⚠️ Stub | `LinearState` enum defined but linear tracking appears minimal |
| Bidirectional inference | ❌ Not implemented | Uses HM-style forward inference only |
| Effect inference | ❌ Not implemented | Effects stored but not propagated through call graph |
| Trait bound checking | ❌ Not implemented | |
| Implied bounds | ❌ Not implemented | |
| Exhaustive pattern checking | ❌ Not implemented | |

---

## 6. MIR — `mir.rs` (927 lines)

### Spec Requirement (§12.2)
CFG-based, ownership/borrows/drops explicit, Polonius-ready.

### Implementation Status

| Feature | Status | Notes |
|---|---|---|
| MirModule/MirFunction/BasicBlock | ✅ Implemented | Correct structure |
| Instruction set | ✅ Implemented | ConstInt, ConstStr, ConstBool, Move, LinearMove, Print, Drop, DropLinear, Jump, JumpIf, Label, BinaryOp, UnaryOp, Return, Assign, Call, FieldAccess, StructAccess, IndexAccess, StructDef, EnumDef |
| AST→MIR lowering (`lower_program_to_mir`) | ✅ Implemented | Handles Let, LetLinear, Print, If, Loop, While, For, Return, Break, Continue, Assign, Block, Struct, Enum, Fn |
| Control flow (jumps/labels) | ✅ Implemented | Loop/while generate jump/label pairs |
| Expression lowering | ✅ Implemented | Number, String, Bool, Var, BinaryOp, Call, FieldAccess |
| Linear move tracking | ✅ Implemented | LetLinear generates LinearMove + DropLinear |
| Scope-based drop insertion | ✅ Implemented | Tracks scopes, inserts Drop at scope exit |
| `format_mir()` | ✅ Implemented | Text dump of MIR |
| CFG construction | ⚠️ Implicit | BasicBlocks exist but no explicit CFG edges |
| Liveness analysis | ❌ Not implemented | |
| Borrow tracking in MIR | ❌ Not implemented | No borrow/reference instructions |
| Effect-annotated MIR | ❌ Not implemented | |

---

## 7. MIR Optimizer — `mir_optimize.rs` (395 lines)

### Implementation Status

| Feature | Status | Notes |
|---|---|---|
| Constant folding | ✅ Implemented | Int arithmetic and comparison at MIR level |
| Dead code elimination | ✅ Implemented | Removes unused variables |
| Simple inlining | ✅ Implemented | Inlines single-block functions |
| Copy propagation | ❌ Not implemented | |
| Common subexpression elimination | ❌ Not implemented | |

---

## 8. Borrow Checker — `polonius.rs` (818 lines)

### Spec Requirement (§5.2, §12.3)
Polonius algorithm for borrow checking on MIR.

### Implementation Status

| Feature | Status | Notes |
|---|---|---|
| RegionInfo | ✅ Implemented | Start/end block+instr, lifetime, parent, universal flag |
| LoanInfo | ✅ Implemented | Name, region, borrower, kind (Shared/Exclusive/Mutable) |
| `export_polonius_input()` | ✅ Implemented | Serializes MIR to text format |
| `check_mir()` | ✅ Implemented | Basic borrow checking on MIR |
| `run_full_analysis()` | ✅ Implemented | Comprehensive ownership/borrow analysis |
| Use-after-move detection | ✅ Implemented | |
| Double-drop detection | ✅ Implemented | |
| Conflicting borrow detection | ✅ Implemented | Shared vs mutable |
| Polonius engine integration | ❌ Not implemented | Uses custom analysis, not `polonius-engine` crate |
| Field projection tracking | ❌ Not implemented | |
| Datalog-based analysis | ❌ Not implemented | |

---

## 9. Codegen Pipeline

### `codegen.rs` (24 lines)
- Selector between Cranelift (default) and LLVM (feature-gated) backends.
- ✅ Correctly structured with `#[cfg(feature = "use_llvm")]`.

### `codegen_lir.rs` (270 lines)
- ✅ Full MIR→LIR lowering for: ConstInt, ConstBool, ConstStr (as zero), Move, BinaryOp, Jump, JumpIf, Return, Call, Print.
- ✅ Variable slot assignment and label patching.
- ⚠️ Strings lowered as zero placeholder (not real string support).

### `codegen_rust.rs` (209 lines)
- ✅ Emits Rust source from MIR, compiles with `rustc`, runs the binary.
- Used as fallback when MIR contains string operations, structs, or enums that LIR can't handle.
- ⚠️ All variables declared as `Option<&str>` — very limited type fidelity.

### `lir` crate (98 lines)
- ✅ Well-defined stack-based IR: `Const`, `Add`, `Sub`, `Mul`, `Div`, `Load`, `Store`, `Call`, `Ret`, `Jump`, `CondJump`, `Drop`, `Nop`.
- ✅ Clean `Module`/`Function`/`Instr` types with `Type::I64`, `Type::Void`, `Type::Ptr`.
- ⚠️ Very minimal — no string type, no struct type, no heap allocation instructions.

### `codegen-cranelift` (934 lines)
- ✅ Full LIR interpreter (stack-based, with locals, calls, conditionals).
- ✅ `compile_and_run_with_jit()` — interprets LIR module and returns results.
- ❌ **Not actual Cranelift JIT** — it's a software interpreter named misleadingly. No `cranelift-codegen` dependency.

### `codegen-llvm` (850 lines)
- ✅ Emits C source from LIR, compiles with Clang, runs binary.
- ✅ Stack simulation in C: `long long stack[1024]`, `long long locals[256]`.
- ⚠️ **Not actual LLVM IR/inkwell** — transpiles to C and uses Clang.

### `codegen-wasm` (337 lines)
- ✅ Real WASM codegen using `wasm_encoder` crate.
- ✅ Emits valid WebAssembly modules with proper type/function/export sections.
- ✅ Handles: constants, arithmetic, locals, jumps, calls, print (via `host_print` import).
- ✅ **Best-quality backend** — produces actual target-specific output.

### `codegen-mlir` (501 lines)
- ✅ MLIR textual representation generation from LIR.
- ✅ Dialect definitions: Func, Arith, Cf, MemRef, Linalg.
- ✅ Operation types: FuncOp, Return, Call, Constant, Add/Sub/Mul/Div, Branch, CondBranch, Block, Print.
- ⚠️ **Textual output only** — no MLIR binary emission or actual MLIR toolchain integration.

---

## 10. Interpreter — `interpreter.rs` (1283 lines)

### Implementation Status

| Feature | Status | Notes |
|---|---|---|
| Value types | ✅ | Int, Str, Bool, Vector, Map, Channel, CancellationToken |
| Expression evaluation | ✅ | All basic expressions |
| Function calls (user-defined) | ✅ | Recursive, with env cloning |
| Built-in functions | ✅ | print, println, len, push, pop, get, set, insert, contains, remove, keys, values, type_of, to_string, to_int, range, map, filter, reduce, sort, assert, assert_eq, panic, format |
| Match expressions | ✅ | Patterns: Wildcard, Literal, Var, Struct, Or |
| Control flow | ✅ | If/else, loop, while, for, break, continue, return |
| Struct instantiation | ✅ | As Map values |
| Channel operations | ✅ | Send, receive |
| Cancellation tokens | ✅ | Create, cancel, check |
| String interpolation | ✅ | InterpolatedFragment evaluation |
| Field assignment | ✅ | `obj.field = value` |
| Index access | ✅ | Vector and Map indexing |
| Closures/lambdas | ❌ | Not supported |
| Async/await execution | ❌ | Not supported |
| Effect handler execution | ❌ | Not supported |

---

## 11. VM — `vm.rs` (314 lines)

### Implementation Status
- ✅ Executes MIR instructions directly (ConstInt, ConstStr, ConstBool, Move, Print, Drop, Jump, JumpIf, BinaryOp, Return, Assign, Call).
- ✅ Simple label-based control flow.
- ⚠️ Only handles `Value::Int`, `Value::Str`, `Value::Bool`.
- ❌ No support for structs, enums, closures, or complex types.
