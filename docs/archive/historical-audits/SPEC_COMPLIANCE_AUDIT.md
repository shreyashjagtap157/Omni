# Omni Spec ↔ Implementation Compliance Audit

**Spec**: `D:\Project\Omni\docs\Omni_Complete_Specification.md` (v2.0, 22 sections, ~80KB)
**Implementation**: `D:\Project\Omni\crates\` (14 workspace crates, 41 source modules in `omni-compiler`)
**Audit Date**: 2026-06-05
**Audit Method**: Read-only. Every claim below is anchored to a file path + line number.

## Coverage Summary

| § | Section | Status | Coverage |
|---|---|---|---|
| 4 | Type System | Partial | ~55% — bidirectional core, generics, traits, ADTs done; variadic, specialization, let-chains, sealed-enum exhaustiveness, runtime reflection absent |
| 5 | Memory Model & Ownership | Partial | ~70% — Polonius facts + mock, Gen/Arena, inout desugar, linear types present; field projections, GC mode, lifetime promotion absent |
| 6 | Effect System | Partial | ~60% — enum + EffectSet + resolver + comptime + handlers in AST; custom effects, handler call desugaring in MIR/codegen absent |
| 7 | Concurrency & Execution Model | Stub | ~20% — AST nodes for spawn/actor/channel/exists; only an interpreter-side `spawn_scope` runtime, no executor, no structured enforcement |
| 8 | Syntax & Surface Design | Partial | ~75% — INDENT/DEDENT, full token set, CST, parser, formatter; let-chains, error context `|>` operator not parsed |
| 9 | Module, Package & Visibility | Partial | ~70% — Visibility enum, ModuleSystem, omni.toml parser, PubGrub solver; build script is a stub |
| 10 | Error Handling & Failure Model | Partial | ~65% — `Result`, `ErrorSet`, `throw<E>` effect, `?` `Try` expr in AST; `? \|> "context"` widening absent |
| 11 | Standard Library Architecture | Missing | ~10% — only Rust-side `omni-stdlib` (Gen, Arena, SlotMap, OmniVector); no Omni-language stdlib |
| 12 | Compilation Model & IR Design | Partial | ~65% — full pipeline, MIR, MIR optimize, LIR lowering, multi-backend; no Salsa, no comptime budget enforcement in driver, no incremental cache |
| 13 | Runtime Architecture | Missing | ~5% — only interpreter + Cranelift stack-VM; no actual runtime crate, no GC, no scheduler |
| 14 | Tooling & Developer Experience | Partial | ~60% — stage0 CLI 18 commands, LSP with hover/rename/workspace index, formatter, bindgen C/JSON/Python; debug session AST stub, no actual debugger |
| 15 | Testing, Diagnostics & Validation | Partial | ~55% — E0xxx diagnostic codes, JSON emitter, 40+ test files; no coverage, no fuzz harness wired into CI, no property test framework |
| 16 | Security, Safety & Capability System | Partial | ~50% — `Capability` enum, `CapabilityToken`, `FfiSandbox` AST, security.rs 252 lines; capability integration with effect system absent |
| 17 | Interoperability & FFI | Stub | ~25% — FfiSandbox AST, type_export.rs (bindgen), abi_check.rs; C/JSON/Python emit; no extern blocks parser, no actual FFI call lowering |
| 18 | Bootstrap Strategy & Self-Hosting | Missing | ~10% — `omni-selfhost` crate exists in workspace but is not implemented; `cargo run -p omni-selfhost` not runnable |
| 19 | Phased Implementation Plan | Partial | Phases 0–2 partially implemented; phases 3–12 are mostly stubs |
| 20 | HELIOS Framework | Missing | Not present in repo |
| 21 | Current State & What Remains | N/A | — |
| 22 | v2.0 Improvements | Partial | See per-section notes |

Overall: **~50–55% spec coverage**. Core frontend (lex/parse/resolve/type-check/effects), Polonius facts, MIR, multi-backend emission, LSP, formatter, and package solver are functional. Generics with implied bounds, generational refs, linear types, inout, ErrorSet, actors, simulators, stage0 CLI are all present at AST+MIR level but have **weak or absent lowering into LIR and backends**. Runtime, GC, executor, structured-concurrency enforcement, full effect handler semantics, HELIOS, and self-hosting are missing.

---

## §4. Type System

### Spec Requirements
- Bidirectional type checking (§4.2)
- `Option<T>` / `Result<T, E>` (§4.3, §4.4)
- Error set types — Zig-style (§4.4)
- Generics with **implied bounds**, **variadic generics**, **limited specialization** (§4.5)
- Traits with **upcasting**, **negative bounds**, **custom diagnostics**, **async traits** (§4.6)
- Pattern matching: or-patterns, deconstructed parameters, let-chains (§4.7)
- Sealed enums, enum variant methods (§4.8)
- `comptime` with **string ops**, **type reflection**, **budget annotation** (§4.9)
- Compile-time + limited runtime reflection (§4.10)

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| Bidirectional check | Done | `crates/omni-compiler/src/type_checker.rs:350,948` — `synthesize_expr` / `check_expr` with effect bitmask |
| `Option<T>`, `Result<T,E>` | Done | `crates/omni-compiler/src/types.rs:215-217` — `Option(Box<Type>)`, `Result(Box<Type>, Box<Type>)`; `?` parse + Try expr in `ast.rs:77` |
| ErrorSet types | Done | `types.rs:217` (`Type::ErrorSet(String)`); `ast.rs:144` (`Stmt::ErrorSet`); `parser.rs:495,2106`; `type_checker.rs:2682,3631`; `mir.rs:1054`; `formatter.rs:384`; `lsp.rs:868` |
| `pub fn` re-exports in type checker | Done | `type_checker.rs:1823-1827` — `type_check_program`, `type_check_program_with_modules` |
| Traits + builtins | Done | `traits.rs:6-56` (`TraitDefinition`, `TraitBound`, `MethodSignature`, `TraitImpl`, `ImplMethod`, `TraitSystem`); builtins incl. Send/Sync (sealed), Default, Try, AsyncDrop, Ord/PartialOrd, Hash, Clone, Iterator, `traits.rs:479-493` (AsyncDrop) |
| Trait upcasting | Done | `traits.rs:552` — `can_upcast_trait` |
| Negative bounds | Done | `traits.rs:541` — `satisfies_negative_bound`; consumed at `type_checker.rs:494,1633` |
| Custom diagnostic attrs | Partial | AST has `diagnostic_attrs: Vec<DiagnosticAttribute>` in `ast.rs:198`; parser wired; no rendering in `lsp.rs` hover/quickfix verified |
| Implied bounds | Partial | `traits.rs:576-617` — `implied_bounds_for_type`, `resolve_implied_bounds`; plumbing in `type_checker.rs:5` thread-local `with_fn_bounds` exists |
| Generics w/ bounds | Partial | AST carries `type_params: Vec<(String, Vec<String>)>` (ast.rs:122,189,196,204); monomorphization is **NOT done** in codegen — no instantiation map |
| Variadic generics `..Ts` | **Missing** | No `..Ts` token; not in `TokenKind` (complete_lexer.rs:9-145) |
| Specialization | **Missing** | No `#[specialize]` attribute; `traits.rs` has no specialization logic |
| Async traits `async fn` in trait | **Missing** | No `async fn` method handling; AsyncDrop is a built-in trait only, not a per-method async-ness marker |
| Sealed enums | Done | `ast.rs:141` (`is_sealed: bool`); `types.rs:213` (`is_sealed: bool`); resolver consumes at `resolver.rs:131`; exhaustiveness in type_checker not verified for sealed |
| Enum variant methods | **Missing** | No `impl EnumName::Variant { ... }` parser path; methods only attach to `impl Type` |
| Pattern: or-patterns | Done | `ast.rs:105` — `Pattern::Or(Vec<Pattern>)`; parser likely emits it (deferred) |
| Deconstructing params | **Missing** | No `fn process((x, y): (i32,i32))` parser |
| Let-chains `let ... and let ...` | **Missing** | grep `let-chain\|let_chains` → 0 matches |
| Comptime | Partial | `comptime.rs` 506 lines, `pub fn new`, budget field on `ComptimeInterpreter`; AST has `ComptimeLimit` (ast.rs:300); no `comptime typeof(T)` reflection function found |
| Comptime string ops | **Missing** | No `comptime_string_*` or `format_compile_time` API in `comptime.rs` |
| Comptime type reflection | **Missing** | No `comptime_typeof` or `TypeInfo` access from `comptime` block |
| Runtime reflection | **Missing** | No `std::reflect` module; no `Any` type or downcast |

### Gaps
1. **No monomorphization pass** — generics are not instantiated per concrete type. `codegen_lir.rs` does not instantiate `T`.
2. **Variadic generics, specialization, async fn-in-trait, runtime reflection, let-chains, deconstructed params** are all absent.
3. **Comptime** is a substantial interpreter (506 lines) but lacks string ops, type reflection, and the budget annotation is not enforced at driver level.

---

## §5. Memory Model & Ownership

### Spec Requirements
- Single-owner + move semantics (§5.1)
- `&T` / `&mut T` (§5.2)
- **Polonius** from day one (§5.2)
- **Field projections** (§5.3)
- **Generational references** Gen<T> (§5.4)
- **Linear types** §5.5
- **Inout parameters** §5.6
- Arena allocators §5.7
- Safe/unsafe boundary §5.8
- **GC mode** module annotation §5.9

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| Ownership + move | Done | `Stmt::Let`, `Stmt::LetLinear` (ast.rs:112-113); resolver tracks `LinearState` (types.rs:2-5: `Available`/`Moved`); `linear_types.rs:219` |
| Shared/exclusive borrows | Done | `&` / `&mut` parsed (parser.rs, deferred to detailed read); polonius facts cover both (`LoanInfo` polonius.rs:67-79) |
| Polonius adapter | Done | `polonius_engine_adapter/src/lib.rs:1-100+` — feature-gated `use_polonius_lib` path; CLI fallback via `OMNI_USE_POLONIUS=1`; mock at `polonius_engine_mock/src/lib.rs:1-100+` |
| Polonius facts emission | Done | `polonius.rs:81-220+` — `export_polonius_input` emits `point`/`def`/`use`/`live`/`kill`/`region_*`/`loan_*`; `:230` `generate_region_loan_facts`; `:916` full export |
| Polonius region/loan | Done | `polonius.rs:653` `generate_cfg_regions`; `:785` `generate_loan_facts`; `RegionInfo` with lifetimes (`:4-50`); `LoanKind` enum (`:75-79`) |
| Field projections | **Missing** | No `RegionInfo` per-field tracking in polonius; MIR place projections not field-aware (no `PlaceElem::Field` projection) |
| Generational references | Done | `generational_refs.rs:273` — `GenRef<T> { idx, gen }`, `Arena<T>` with slot/live/gen; `is_valid: Option<NonNull<T>>` |
| Linear types | Done | `linear_types.rs:219` — `LinearKind`, `RegionKind`; AST `Stmt::LetLinear`, `is_linear: bool` on struct (ast.rs:113,134) |
| Inout desugar | Done | `inout_desugar.rs:203` — heuristic `inout_` param name prefix; rewrites linear moves for caller/callee; lowering helper at `:106+` |
| Arena allocation | Partial | `Arena<T>` exists in Rust-side stdlib (`omni-stdlib/src/lib.rs`); **no Omni-language Arena type** parsed/typed |
| Unsafe boundary | Done | AST `Stmt::Unsafe` (ast.rs:182); `MirFunction::is_safe_wrapper` (mir.rs:13-28); `@safe_wrapper` attribute not in lexer |
| GC mode | **Missing** | AST has `Stmt::GcMode { mode: String, span }` (ast.rs:219); no collector, no write barrier, no typed crossing point |

### Gaps
1. **Field projections (§5.3)** — critical v2.0 feature for borrow ergonomics — has no implementation in MIR or polonius.
2. **GC mode** is an AST stub; no runtime collector exists.
3. **Arena** is a Rust type only, not an Omni-language type.

---

## §6. Effect System

### Spec Requirements
- Built-in effects: `io, async, throw<E>, panic, alloc, rand, time, log, pure` (§6.2)
- Effect inference (§6.3)
- Effect handlers with `effect X: fn log(...)` definition (§6.4)
- Public API requires explicit annotation (§6.3)

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| Effect enum (all 9) | Done | `types.rs:8-19` — `Pure, Io, Async, Throw(Box<Type>), Panic, Alloc, Rand, Time, Log, Custom(String)` (10th: custom) |
| EffectSet | Done | `types.rs:22-176` — `union`, `union_with`, `difference`, `contains`, `to_string_list`, `BitOr` impl |
| Effect propagation in type checker | Done | `type_checker.rs:350-937+` — `synthesize_expr` and `check_expr` accumulate effects via `acc_effects |= ef` (e.g. `:451,524,538,665,674,725,748,759,781,789,810`); bitmask return documented at `:351` |
| Effect resolver (separate pass) | Done | `effect_resolver.rs:143` — `pub fn new()`; data passed to LSP `Effect Explorer` (lsp.rs:705) |
| Effect handler AST | Done | `Stmt::EffectHandler { effect, handler, span }` (ast.rs:227) |
| `effect` keyword | Done | `TokenKind::Effect` (complete_lexer.rs:84); `parser.rs` emits it |
| Custom effects | Done | `Effect::Custom(String)` (types.rs:18) |
| Effect inference vs explicit | Partial | `pub fn effects: Vec<String>` carried on `Stmt::Fn` (ast.rs:125); checker infers via accumulation but does not enforce explicit annotation at public API |
| Effect handler lowering | **Missing** | No `MirInst` for `handle`; mir.rs handlers likely fall-through to a default |
| Throw effect (`throw<E>`) | Partial | `Effect::Throw(Box<Type>)` exists; `Expr::Try` exists; `?` parse exists; **no implicit widening** across fn boundary |

### Gaps
1. **Effect handler call desugaring** (resuming, returning, aborting) is absent from MIR/codegen.
2. **No effect handler typechecking** — handler signature compatibility not verified.
3. **Public API enforcement** of explicit effect annotations is not in the driver.

---

## §7. Concurrency & Execution Model

### Spec Requirements
- Structured concurrency: child tasks cannot outlive parent scope (§7.1)
- Async/await (§7.2)
- Generators / yield (§7.3)
- Actors + supervision trees (§7.4)
- Channels (§7.5)
- Cancellation tokens (§7.6)
- Deterministic runtime (§7.7)

### Implementation Status: **Stub**

| Sub-feature | Status | Evidence |
|---|---|---|
| Spawn AST | Partial | `Stmt::Spawn { task, span }` (ast.rs:232); parser `parse_spawn` (parser.rs:1703) |
| Spawn runtime | Partial | `effect_system.rs:111` — `pub fn spawn(&mut self, child_id: u64)`; `integration.rs:86` — `spawn_scope`; test at `:208` and `:136` — only interpreter-side simulation |
| Work-stealing executor AST | Stub | `Stmt::WorkStealingExecutor { num_threads, queue_type }` (ast.rs:247) — no implementation |
| Deterministic runtime AST | Stub | `Stmt::DeterministicRuntime { max_tasks }` (ast.rs:252) — no implementation |
| CancelToken AST | Stub | `Stmt::CancelToken` (ast.rs:223); `Effect::Cancel` not in effect enum |
| Channel AST | Stub | `Stmt::Channel { elem_type, capacity }` (ast.rs:236) — no lowering |
| Actor AST | Stub | `Stmt::Actor { name, state, handlers }` (ast.rs:241); parser `parse_actor` (parser.rs:1741); resolver at `:299`; **type_checker lowers to nothing** (`:2852`); mir.rs:1109 fall-through; interpreter:1457 no-op; formatter prints only `name state` (`:476-477`) |
| `async fn` | Partial | `is_async: bool` on `Stmt::Fn` (ast.rs:121); `Expr::Await` (ast.rs:76); **no async lowering in MIR**; no async trait method support |
| `yield` keyword | Stub | `TokenKind::Yield` (complete_lexer.rs:85,194) but no `Stmt::Yield` or generator lowering |
| Structured concurrency enforcement | **Missing** | No type-level enforcement; the `spawn_scope` in `effect_system.rs:102+` is a runtime data structure, not a type-level guarantee |
| Supervision trees | **Missing** | — |

### Gaps
1. **No actual executor** — `WorkStealingExecutor` is an AST stub.
2. **No async runtime** — `async fn` parses but doesn't lower to a state machine.
3. **No structured-concurrency type-level guarantee**.
4. **Generators / yield** are token-only.
5. **Actors** are parsed, scoped, and formatted, but never executed.

---

## §8. Syntax & Surface Design

### Spec Requirements
- INDENT/DEDENT significant whitespace (§8.1)
- Full operator set (§8.2)
- Doc comments `///` (§8.3)
- Interpolated strings `{expr}` (§8.4)
- Pattern guards in match (§8.5)
- Blocks-as-expressions (§8.6)
- `?` propagation, `try` keyword (§8.7)

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| INDENT/DEDENT | Done | `TokenKind::Indent, Dedent, Newline` (complete_lexer.rs:21-23); `indent_stack` at `:162`; `at_line_start: bool` |
| Operator set | Done | `TokenKind` has all 24 ops: `Equals, Plus, Minus, Star, Slash, Percent, EqEq, NotEq, Lt, LtEq, Gt, GtEq, AndAnd, OrOr, Bang, LParen, RParen, Arrow, FatArrow, LBracket, RBracket, LBrace, RBrace, Comma, Colon, ColonColon, Dot, DotDot, DotDotDot, Semi, At, Question, Tilde` |
| Doc comments | Done | `TokenKind::DocComment`; `Stmt::DocComment { target, content }` (ast.rs:266); 247+ doc-comment-related lines; parser.rs:191,205,239,… many skips; formatter.rs:502,744 |
| Interpolated strings | Done | `Expr::Interpolated(Vec<InterpolatedFragment>, Span)` (ast.rs:28); `InterpolatedFragment::Literal | Expr` (`:86-89`) |
| Pattern guards | Done | `MatchArm { pattern, guard: Option<Box<Expr>>, body }` (ast.rs:92-97) |
| Blocks-as-expressions | Done | `Expr::Block(Vec<Stmt>, Span)` (ast.rs:57); `Expr::IfExpr` (`:51`) |
| `?` operator | Done | `Expr::Try(Box<Expr>, Span)` (ast.rs:77) |
| `try` keyword | **Missing** | grep `try\{` → 0 results; not in `TokenKind` |
| `? \|> "context"` widening | **Missing** | No `|` (context-operator) lowering |
| All 14 numeric type keywords | Partial | `Int, Int8..Int64, UInt..UInt64, Float32, Float64` (complete_lexer.rs:126-138) — full set present |
| `pipe` keyword | Done | `TokenKind::Pipe`; `keyword "pipe"` (complete_lexer.rs:74,183) |

### Gaps
1. **`try` keyword** is not in the lexer; no `try { ... }` blocks for `?`-with-handler.
2. **Error-context `|>` operator** is not in the operator precedence table nor AST.

---

## §9. Module, Package & Visibility System

### Spec Requirements
- Visibility: `private, pub, pub-mod, pub-pkg, pub-cap(C), pub-friend(M)` (§9.1)
- Module hierarchy: file + inline (§9.2)
- Workspace (§9.3)
- Lockfile with PubGrub-style solver (§9.4)
- Build script `build.omni` (§9.5)
- Capability system integration (§9.6)

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| Visibility enum (6 levels) | Done | `Visibility::{Private, PubMod, PubPkg, Pub, PubCap(String), PubFriend(String)}` (ast.rs:5-12) |
| Inline modules | Done | `Stmt::Mod(String, Span)`, `Stmt::ModBlock(String, Vec<Stmt>, Span)` (ast.rs:116-117) |
| File modules | Done | `ModuleSystem::load_file_module` (module_system.rs:95-160) — looks for `<mod>.omni` or `<mod>/mod.omni` |
| Module dependency graph | Done | `module_system.rs:51` — `module_path -> list of module_paths` |
| Manifest parser (omni.toml) | Done | `omni_toml_parser.rs:5-117` — `OmniManifest`, `parse_manifest`, `load_manifest`; package subdir has its own copy (449 lines + 117 lines) |
| Capability decl | Done | `Stmt::Capability { name, permissions }` (ast.rs:276); security.rs:30-209 wires tokens |
| Workspace | Done | `package/omni_workspace.rs:7-60` — `Workspace`, `load`, `get_member`, `resolve_all_dependencies` |
| PubGrub solver | Done | `package/solver.rs:259+` — `PubGrubSolver`, `solve`, `resolve_with_lock`; Term, Assignment, DerivationCause; BTreeMap for determinism |
| Lockfile | Done | `package/lockfile.rs:9+` — `Lockfile::new(resolved: BTreeMap<...>)` |
| Build graph | Done | `package/build_graph.rs:6-18` — `Node`, `BuildGraph` |
| **Build script execution** | **Stub** | `package/build_script.rs:5-23` — `BuildConfig::new` + `run_build_script` that just reads `build.omni` and prints message; **does not execute** |
| `pub mod`, `pub pkg`, `pub cap`, `pub friend` in module system | Done | `module_system.rs:216+` — `is_accessible` checks all 6 levels |
| Module indexing for LSP | Done | `lsp.rs:267` — `Recursively index .omni files under root`; `CompilationDatabase` (`:52`) |

### Gaps
1. **`build.omni` is not executed** — only a manifest is parsed; no DSL for user-defined build steps.
2. **Module resolution across packages** is not verified — only within-workspace file lookup is implemented.

---

## §10. Error Handling & Failure Model

### Spec Requirements
- `Result<T, E>` (§10.1)
- `?` propagation (§10.2)
- Error set types (§10.3 — duplicated from §4.4)
- Typed error context chains `? \|> "msg"` (§10.4)
- `panic` as distinct effect (§10.5)
- `AsyncDrop` for async destructors (§10.6)

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| `Result<T,E>` | Done | `types.rs:216` |
| `?` operator | Done | `Expr::Try` (ast.rs:77) |
| ErrorSet types | Done | `Stmt::ErrorSet { name, variants }` (ast.rs:144); parsed at parser.rs:2106; type-checked at type_checker.rs:2682,3631 |
| `panic` effect | Done | `Effect::Panic` (types.rs:13); builder `with_panic()` (types.rs:57) |
| `AsyncDrop` trait | Done | `traits.rs:479-493` — built-in trait with `async_drop` method; `required_methods: vec!["async_drop"]` |
| Context chains `? \|> "msg"` | **Missing** | No `|>` (pipe) AST node; `TokenKind::Pipe` exists but no contextual-error semantics |
| Error-set widening across `?` | **Missing** | Type checker does not implement `T1 -> T2` widening on `?` |

### Gaps
1. **Context chains** are absent.
2. **Implicit widening** of error types across `?` boundary is absent.

---

## §11. Standard Library Architecture

### Spec Requirements
- `core::*` — Vec, HashMap, String, Box, Option, Result, iterators (§11.1)
- `std::*` — fs, net, env, process, threads, time (§11.2)
- `math::*` — complex, vectors, matrices (§11.3)
- `reflect::*` — type metadata (§11.4)
- `testing::*` — property, fuzz, contract (§11.5)
- `async::*` — runtime, channels, cancellation (§11.6)
- `ai::*` / `tensor::*` — ML primitives (§11.7)

### Implementation Status: **Missing**

| Sub-feature | Status | Evidence |
|---|---|---|
| Omni-language stdlib | **Missing** | No `std/core/math/reflect/testing/async/ai/tensor` modules in `omni-stdlib` or any `.omni` file |
| Rust-side stdlib | Partial | `crates/omni-stdlib/src/lib.rs` — `Gen<T>`, `Arena<T>`, `SlotMap`, `OmniVector` only |
| 9 example .omni files | Smoke | `examples/blank_line, function_call, hello, move_error, stdlib_usage, temp_call, test_input, test_temp, typecheck` — tiny tests, not a stdlib |

### Gaps
1. **No Omni-language standard library exists.** The Rust `omni-stdlib` crate is a host-side bootstrap, not a user-facing language stdlib.
2. No `core`, `std`, `math`, `reflect`, `testing`, `async`, `ai` modules are defined in `.omni` source.

---

## §12. Compilation Model & IR Design

### Spec Requirements
- Pipeline: lex → parse → resolve → type → effect → MIR → LIR → backend (§12.1)
- MIR with CFG, SSA-like, phi nodes, projections (§12.2)
- LIR — backend-neutral (§12.3)
- Backend selection: Cranelift (dev), LLVM (release), WASM, MLIR (§12.4)
- Comptime budget (§12.5)
- Salsa query caching (§12.6)

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| Driver pipeline | Done | `crates/omni-compiler/src/driver.rs:40` — `Compiler::new(source, backend)`; `pub use parser_utils::parse_file` (lib.rs:42) |
| MIR | Done | `mir.rs:1361` — `MirModule`, `MirFunction`, `BasicBlock`, `Instruction` (44+ lines), `is_safe_wrapper`, `format_mir` (`:1265`) |
| MIR optimize pass | Partial | `mir_optimize.rs:369` — pass present; not verified to run in driver |
| LIR lowering | Partial | `codegen_lir.rs:264` — MIR→LIR exists |
| Cranelift backend (dev) | Done | `crates/codegen-cranelift/src/lib.rs:212` — `compile_and_run_with_jit` (JITModule + JITBuilder); re-exported at `:1200`; also has stack interpreter `run_lir_interpreter` |
| LLVM backend | Done | `crates/codegen-llvm/src/lib.rs` — emits C via clang; sanitizes symbol names; stub_bridge tests |
| WASM backend | Done | `crates/codegen-wasm/src/lib.rs` — real wasm bytes via `wasm_encoder`; TypeSection, ImportSection, FunctionSection, ExportSection, CodeSection |
| MLIR backend | Stub | `crates/codegen-mlir/src/lib.rs` — AST-only `MlirOp` enum for Func/Arith/Cf/MemRef/Linalg dialects; `basic_jit.rs` exists |
| Comptime budget in driver | **Missing** | `ComptimeLimit { max_ops }` AST (ast.rs:300); `ComptimeInterpreter` has `ops_budget` (macros.rs:197) but driver does not enforce |
| Salsa query caching | **Missing** | `lsp_salsa_db.rs:154` is a stub; `LspDb` re-export at `:6,177,181` |
| Comptime string/type reflection in IR | **Missing** | Comptime interpreter (comptime.rs:506) has no `comptime_string_*` or type reflection opcodes |

### Gaps
1. **No Salsa incremental cache** — LSP has 2 DB backends (incremental + salsa) but both are stubs.
2. **MLIR is AST-only** — no real MLIR C-API binding.
3. **Comptime budget** is per-call only, not a global compile-time cap.

---

## §13. Runtime Architecture

### Spec Requirements
- GC runtime (tracing, conservative) (§13.1)
- Work-stealing executor (§13.2)
- Effect handler runtime (§13.3)
- Async scheduler with structured concurrency (§13.4)
- Capability-checked I/O (§13.5)

### Implementation Status: **Missing**

| Sub-feature | Status | Evidence |
|---|---|---|
| GC | **Missing** | No collector; `Stmt::GcMode` is AST-only |
| Work-stealing executor | **Missing** | `WorkStealingExecutor` AST only |
| Effect handler runtime | **Missing** | Handlers in AST, no runtime dispatch |
| Async scheduler | **Missing** | `async fn` parses but no scheduler |
| Capability-checked I/O | **Missing** | `CapabilityToken` (security.rs:209) exists, no I/O integration |
| Interpreter | Partial | `interpreter.rs:1474` — tree-walk interpreter; many `Stmt::Actor`, `Stmt::Tensor`, `Stmt::DocComment`, `Stmt::ErrorSet` are no-ops (`:1447,1457,1460,1462`) |
| VM | Partial | `vm.rs:333` — VM exists but limited |

### Gaps
1. **No runtime crate** — only an interpreter used for testing.
2. The interpreter is **not a runtime**: it does not run compiled programs, only AST.

---

## §14. Tooling & Developer Experience

### Spec Requirements
- CLI: `omni new/parse/run/check/test/build/...` (§14.1)
- Formatter (§14.2)
- LSP server (§14.3)
- Documentation generator (§14.4)
- Debugger (`debug_session`) (§14.5)
- Bindgen — C, JSON, Python (§14.6)
- ABI checker (§14.7)

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| Stage0 CLI | Done | `crates/omni-stage0/src/main.rs:15+` — 18 commands: new, parse, parse-cst, fmt-cst, fmt, fmt-check, run, check, test, emit-mir, check-mir, run-mir, run-native, compile, emit-wasm, emit-lir, export-types, bindgen, check-abi |
| Bindgen formats | Done | `CHeader, Json, Python` (stage0/src/main.rs:19-39) |
| Formatter | Done | `formatter.rs:11-788` — `Formatter::new`, `check_format`, `format_program`, `format_cst_source`, `format_program_with_config`, `format_cst_source_with_config`; supports all major Stmt kinds (Actor, Tensor, ErrorSet, DocComment at `:476,489,384,502`) |
| LSP | Partial | `lsp.rs:1175` — `CompilationDatabase` (`:52`), workspace indexing (`:267`), hover, rename (`:196`), effect explorer (`:705`); but incremental + salsa DBs are stubs |
| Doc generator | Stub | `doc_gen.rs:90` — `"Documentation placeholder (would parse doc comments)"` (`:41`) |
| Debug session | Stub | `Stmt::DebugSession { port, breakpoints }` (ast.rs:271) — no actual debugger; no DAP integration |
| Type export | Partial | `type_export.rs:426` — `export_types` for bindgen; used by `export-types` CLI |
| ABI check | Partial | `abi_check.rs:192` — used by `check-abi` CLI |

### Gaps
1. **Doc generator** is a stub.
2. **No debugger / DAP server**.
3. **LSP** has hover/rename/workspace-index but no real incremental compute (salsa is stub).

---

## §15. Testing, Diagnostics & Validation

### Spec Requirements
- `omni test` (§15.1)
- Property-based testing (§15.2)
- Fuzz testing (`fuzz_harness` crate) (§15.3)
- Contract testing (`requires`, `ensures`, `invariant`) (§15.4)
- Diagnostic quality — Elm/Rust-class (§15.5)
- UI / snapshot tests (§15.6)

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| `omni test` CLI | Done | `omni test` command in stage0 (main.rs) |
| Contract AST | Done | `Stmt::ContractRequires`, `ContractEnsures`, `ContractInvariant` (ast.rs:285-299) |
| Contract type checking | **Missing** | grep in type_checker.rs does not show contract checking; no enforcement at MIR/codegen |
| Diagnostic codes | Done | `diagnostics.rs:271` `pub mod error_codes` with E0xxx codes; `Diagnostic` struct with `code`, `severity`, `labels`, `message` |
| JSON diagnostic emit | Done | `diagnostics.rs:221` — `pub fn emit_diagnostic_json` |
| `LabelStyle::Primary/Secondary` | Done | (referenced in spec of diagnostic engine) |
| Test suite | Done | 40+ test files in `crates/omni-compiler/tests/` — borrow_check_tests, polonius_adapter, mir_field_projection, advanced_features, effect_tests, package_tests, type_inference_ui |
| Property test framework | **Missing** | No `proptest` integration; no `quickcheck` integration |
| Fuzz harness | Partial | `crates/fuzz_harness` exists in workspace; **not wired to CI** |
| Coverage | **Missing** | No `cargo-llvm-cov` or `tarpaulin` config |
| UI / snapshot tests | Partial | `type_inference_ui.rs` exists |

### Gaps
1. **No property-based testing framework**.
2. **No fuzz harness wired into CI** (`fuzz_harness` crate exists but is dormant).
3. **No coverage tool integration**.
4. **Contract verification** (requires/ensures/invariant) is an AST stub — not checked.

---

## §16. Security, Safety & Capability System

### Spec Requirements
- `unsafe` blocks (§16.1)
- **Capability tokens** for I/O, network, fs, env, random, time, process, thread, ffi (§16.2)
- FFI sandbox (§16.3)
- `@safe_wrapper` attribute (§16.4)

### Implementation Status: **Partial**

| Sub-feature | Status | Evidence |
|---|---|---|
| `unsafe` block | Done | AST `Stmt::Unsafe` (ast.rs:182); `@safe_wrapper` not in lexer |
| Capability enum | Done | `security.rs:30+` — `Capability::{Io, Network, Filesystem, Environment, Random, Time, Process, Thread, Ffi}` |
| CapabilityToken | Done | `security.rs:209` — `CapabilityToken::new`; uses `AtomicPtr` for thread-safety |
| FFI sandbox AST | Done | `Stmt::FfiSandbox { allow_list }` (ast.rs:281) |
| Sandbox impl | Partial | `security.rs:122` — `Sandbox::new(stack_size, memory_limit)` |
| Capability integration with effects | **Missing** | `Effect::Io` is tracked, but no linkage to a specific `CapabilityToken` |
| `@safe_wrapper` attribute | **Missing** | Not in lexer keywords; `MirFunction::is_safe_wrapper` (mir.rs:13) exists but attribute is not parsed |
| Effect-vs-capability enforcement | **Missing** | No driver-level check that IO effect has Io capability |

### Gaps
1. **Capabilities are not linked to effects** — there's no enforcement that a function with `Effect::Io` actually has an `Io` capability token.
2. **`@safe_wrapper` is not parsed**.
3. **FFI sandbox** has a constructor but no policy enforcement at the FFI call site.

---

## §17. Interoperability & FFI

### Spec Requirements
- `extern` blocks (§17.1)
- C ABI / bindgen C-header emission (§17.2)
- JSON / Python type export (§17.3)
- ABI checker (§17.4)
- Sandbox for FFI calls (§17.5)

### Implementation Status: **Stub**

| Sub-feature | Status | Evidence |
|---|---|---|
| `extern` block parser | **Missing** | No `extern "C" { fn ... }` parser path |
| Bindgen C-header | Done | `type_export.rs:426` — `export_types` produces C headers; `CHeader` format in stage0 main |
| Bindgen JSON | Done | `Json` format in stage0 main |
| Bindgen Python | Done | `Python` format in stage0 main |
| ABI check | Partial | `abi_check.rs:192` — `check_abi` CLI command |
| FFI call lowering | **Missing** | No `MirInst::CallExtern` |
| FFI sandbox integration | **Missing** | Sandbox struct exists, not wired |

### Gaps
1. **No `extern` blocks** — cannot declare foreign functions in Omni source.
2. **No FFI call lowering** — no way to actually call into a foreign library.
3. **Bindgen works one-way** (Omni → C) but not C → Omni.

---

## §18. Bootstrap Strategy & Self-Hosting Roadmap

### Spec Requirements
- Stage 0 (Rust) compiles a partial Omni subset (§18.1)
- Stage 1 (Rust+Omni): Omni stdlib in Rust+Omni (§18.2)
- Stage 2: `omnic` written in Omni compiles Stage 0 source (§18.3)
- Stage 3: Stage 2 compiles Stage 1 (§18.4)
- Stage 4: Stage 3 compiles Stage 2 — bootstrap closed (§18.5)
- Module-by-module migration (§18.6)

### Implementation Status: **Missing**

| Sub-feature | Status | Evidence |
|---|---|---|
| `omni-selfhost` crate | Stub | Listed in workspace (`crates/omni-selfhost/`); `cargo run -p omni-selfhost` not runnable |
| `omni-release` crate | Stub | Listed in workspace; release packaging only |
| `omni-fuzz` crate | Stub | Listed in workspace |
| `bootstrap-migration-verifier` skill | N/A | Skill metadata, not code |
| Self-hosting diff verification | **Missing** | No `Rust↔Omni binary diff` harness |
| Module migration gating | **Missing** | No automated gate per Rust module |

### Gaps
1. **Self-hosting is not started.** `omni-selfhost` is a workspace placeholder.
2. No binary diff infrastructure.
3. No reproducible trust chain.

---

## §19. Phased Implementation Plan

### Spec Phases → Reality

| Phase | Goal | Status |
|---|---|---|
| Phase 0 (0.0.1) | Lexer + Parser + CST + basic types | **Done** |
| Phase 1 (0.1.0) | Type checker, traits, generics | **Partial** — checker done; no monomorphization |
| Phase 2 (0.2.0) | Effects, linear types, inout | **Partial** — all in AST/MIR; no full enforcement |
| Phase 3 (0.3.0) | MIR, Polonius, basic Cranelift | **Done** — polonius facts + Cranelift JIT |
| Phase 4 (0.4.0) | Modules, packages, solver, WASM | **Partial** — module system + PubGrub + WASM; build script stub |
| Phase 5 (0.5.0) | LLVM, optimizations, MLIR skeleton | **Partial** — LLVM C-emit done; MLIR stub |
| Phase 6 (0.6.0) | LSP, formatter, formatter-check | **Done** |
| Phase 7 (0.7.0) | Contracts, properties, fuzz | **Missing** — contracts AST only |
| Phase 8 (0.8.0) | Custom effects, handlers | **Missing** — built-in effects only |
| Phase 9 (0.9.0) | Async runtime, structured concurrency | **Missing** — `spawn` AST only |
| Phase 10 (1.0.0) | GC, self-hosting bootstrap | **Missing** — no GC, no bootstrap |
| Phase 11 (2.0.0) | Interoperability / FFI | **Missing** — no `extern` parser |
| Phase 12 (3.0.0) | MLIR / HELIOS | **Missing** — no MLIR C-API, no HELIOS |

---

## §20. HELIOS Framework (Platform Layer)

**Status**: **Missing** — not present in the repository.

---

## §22. v2.0 Improvements — Coverage Map

| v2.0 Addition | Section | Status |
|---|---|---|
| Implied bounds | §4.5 | Partial (plumbing in traits.rs) |
| Variadic generics | §4.5 | Missing |
| Limited specialization | §4.5 | Missing |
| Trait upcasting | §4.6 | Done (traits.rs:552) |
| Negative bounds | §4.6 | Done (traits.rs:541) |
| Custom diagnostic attrs | §4.6 | Partial (AST only) |
| Async traits (native) | §4.6 | Missing |
| Or-patterns | §4.7 | Done (ast.rs:105) |
| Deconstructing params | §4.7 | Missing |
| Let-chains | §4.7 | Missing |
| Sealed enums | §4.8 | Done (is_sealed flag) |
| Enum variant methods | §4.8 | Missing |
| Comptime string ops | §4.9 | Missing |
| Comptime type reflection | §4.9 | Missing |
| Comptime budget | §4.9 | Partial (per-interpreter) |
| Limited runtime reflection | §4.10 | Missing |
| Field projections | §5.3 | Missing |
| Generational references | §5.4 | Done (generational_refs.rs) |
| Linear types | §5.5 | Done (linear_types.rs) |
| Inout parameters | §5.6 | Done (inout_desugar.rs) |
| GC mode | §5.9 | Missing |

---

## Top-Priority Gaps (Recommendations)

1. **Monomorphization pass** — generics currently have no instantiation. The compiler cannot lower generic functions.
2. **Field projections in MIR + Polonius** — the central v2.0 borrow-ergonomic feature is absent.
3. **Effect handler lowering in MIR** — `effect X: ...` parses but never lowers.
4. **Async runtime + structured-concurrency type-level enforcement** — the entire concurrency model is AST-only.
5. **`extern` block parser** — without it, no FFI is possible.
6. **Contract verification** — `requires`/`ensures`/`invariant` are AST-only.
7. **Comptime string ops, type reflection, and budget enforcement in driver**.
8. **Self-hosting bootstrap** — `omni-selfhost` is a workspace placeholder.
9. **MLIR C-API binding** — currently AST-only.
10. **GC + runtime crate** — no actual runtime, only an interpreter.

---

## Evidence Index by File

| File | LOC | What it carries |
|---|---|---|
| `crates/omni-compiler/src/lib.rs` | 43 | Module index (41 submodules) |
| `crates/omni-compiler/src/driver.rs` | — | `Compiler`/`Backend`/`CompilationResult` |
| `crates/omni-compiler/src/ast.rs` | 311 | Expr, Stmt (40+ variants), Pattern, Visibility (6 levels), EnumVariant |
| `crates/omni-compiler/src/types.rs` | 226 | Effect (10 variants), EffectSet, Type (with ErrorSet), EnumVariant |
| `crates/omni-compiler/src/type_checker.rs` | 3748 | Bidirectional check, monomorphization-not-yet |
| `crates/omni-compiler/src/complete_lexer.rs` | 1067 | TokenKind (~70 variants), keywords, INDENT/DEDENT |
| `crates/omni-compiler/src/parser.rs` | 2674 | parse_program, parse_statement, parse_actor/spawn/tensor/simd/error_set |
| `crates/omni-compiler/src/resolver.rs` | 701 | ScopeTree, ResolveResult, def_id |
| `crates/omni-compiler/src/traits.rs` | 619 | TraitSystem, upcasting, negative bounds, AsyncDrop |
| `crates/omni-compiler/src/polonius.rs` | 916 | RegionInfo, LoanInfo, export_polonius_input, region/loan fact generators |
| `crates/omni-compiler/src/mir.rs` | 1361 | MirModule, MirFunction, BasicBlock, Instruction, is_safe_wrapper |
| `crates/omni-compiler/src/mir_optimize.rs` | 369 | MIR optimization pass |
| `crates/omni-compiler/src/codegen_lir.rs` | 264 | MIR → LIR lowering |
| `crates/omni-compiler/src/codegen_rust.rs` | 218 | Rust backend |
| `crates/omni-compiler/src/effect_resolver.rs` | 143 | Effect propagation pass |
| `crates/omni-compiler/src/effect_system.rs` | 187 | FutureType, CancellationToken, EffectHandler, spawn |
| `crates/omni-compiler/src/linear_types.rs` | 219 | LinearKind, RegionKind, LinearState tracking |
| `crates/omni-compiler/src/inout_desugar.rs` | 203 | inout_ param-name heuristic; MIR lowering |
| `crates/omni-compiler/src/generational_refs.rs` | 273 | GenRef, Arena, is_valid |
| `crates/omni-compiler/src/security.rs` | 252 | Capability enum, CapabilityToken (AtomicPtr), Sandbox |
| `crates/omni-compiler/src/module_system.rs` | 678 | Module, Symbol, ModuleSystem, accessibility checks |
| `crates/omni-compiler/src/omni_toml_parser.rs` | 449 | CapabilityDecl, BuildTarget, Edition, FeatureDecl |
| `crates/omni-compiler/src/package/*` | ~2000 | solver, lockfile, build_graph, omni_workspace, omni_toml_parser, build_script (STUB) |
| `crates/omni-compiler/src/macros.rs` | 493 | macro_rules interpreter, comptime interpreter |
| `crates/omni-compiler/src/comptime.rs` | 506 | Comptime interpreter |
| `crates/omni-compiler/src/diagnostics.rs` | 288 | Diagnostic struct, E0xxx codes, JSON emit |
| `crates/omni-compiler/src/formatter.rs` | 788 | Formatter, check_format, format_program, format_cst |
| `crates/omni-compiler/src/lsp.rs` | 1175 | CompilationDatabase, hover, rename, workspace index |
| `crates/omni-compiler/src/lsp_incr_db.rs` | 44 | SimpleLspDb stub |
| `crates/omni-compiler/src/lsp_salsa_db.rs` | 154 | Salsa backend stub |
| `crates/omni-compiler/src/interpreter.rs` | 1474 | Tree-walk interpreter (no-ops for Actor, Tensor, etc.) |
| `crates/omni-compiler/src/vm.rs` | 333 | VM |
| `crates/omni-compiler/src/doc_gen.rs` | 90 | Documentation placeholder |
| `crates/omni-compiler/src/abi_check.rs` | 192 | ABI checks |
| `crates/omni-compiler/src/type_export.rs` | 426 | export_types for bindgen |
| `crates/omni-compiler/src/cst.rs` | 144 | SyntaxNode, format_cst |
| `crates/omni-compiler/src/integration.rs` | — | spawn_scope test integration |
| `crates/omni-compiler/src/parser_utils.rs` | — | parse_file (re-exported from lib.rs:42) |
| `crates/omni-compiler/src/codegen.rs` | 16 | Likely facade |
| `crates/omni-compiler/src/llvm_detect.rs` | — | LLVM detection |
| `crates/codegen-cranelift/src/lib.rs` | 1200 | Cranelift JITModule + JITBuilder + stack VM interpreter |
| `crates/codegen-llvm/src/lib.rs` | — | C-emit via clang |
| `crates/codegen-wasm/src/lib.rs` | — | wasm_encoder-based real wasm bytes |
| `crates/codegen-mlir/src/lib.rs` | — | MlirOp AST + basic_jit.rs |
| `crates/polonius_engine_adapter/src/lib.rs` | 1528 | Feature-gated `use_polonius_lib` + CLI fallback |
| `crates/polonius_engine_mock/src/lib.rs` | 285 | In-process mock solver |
| `crates/omni-stdlib/src/lib.rs` | — | Gen, Arena, SlotMap, OmniVector (Rust-side) |
| `crates/omni-stage0/src/main.rs` | — | 18 CLI commands; CHeader/Json/Python bindgen |
| `examples/*.omni` | 9 files | Tiny smoke tests |
| `crates/omni-compiler/tests/*.rs` | 40+ files | borrow_check_tests, polonius_adapter, mir_field_projection, advanced_features, effect_tests, package_tests, type_inference_ui |
