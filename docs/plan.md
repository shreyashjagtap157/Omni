## Plan: Omni Implementation Roadmap

TL;DR - Build Omni as a staged, self-hosting compiler by following a strict vertical-slice bootstrap: create a Rust Stage0 toolchain, scaffold a minimal Cargo workspace and stdlib stubs, implement the compiler front end (lexer, layout, CST, parser, formatter), then the semantic core (AST, resolver, type+effect inference), lower to MIR and integrate Polonius for borrow checking, add a LIR/codegen path (Cranelift for dev), and iteratively re-enable and reconcile the real standard library from preserved originals. Verify self-hosting by the Stage1==Stage2 parity requirement (identical binaries).

**Steps**
1. Project Foundation (scaffold) — *depends on nothing*
- **Workspace**: Create a Rust Cargo workspace with separate crates for the Stage0 CLI, compiler library, stdlib stubs, and tooling (LSP, tests).
- **Tooling**: Add basic `fmt`, linter, CI skeleton (single pipeline), and a reproducible-build guide.
- **Deliverable**: reproducible `cargo build --workspace` for the Stage0 toolchain.
- **Acceptance**: Stage0 builds cleanly on local dev machine.

2. Lexical & Layout Front End — *depends on Step 1*
- **Lexer + Layout**: Implement an INDENT/DEDENT-aware lexer and layout engine producing tokens with synthetic INDENT/DEDENT/EOL tokens.
- **CST**: Produce a lossless Concrete Syntax Tree (rowan-like) to drive formatting, diagnostics, and incremental parsing.
- **Formatter**: A CST-based `omni fmt` that round-trips input without semantic change.
- **Acceptance**: `omni parse file.omni` produces CST; `omni fmt` is idempotent on well-formed code.

3. Parser, Recovery, and UI Tests — *parallel with Step 2 after basic lexer works*
- **Parser**: Recursive-descent + Pratt parser producing AST from CST; robust panic-mode recovery to emit diagnostics while continuing parsing.
- **UI tests**: Add a suite of targeted parse-failure UI tests and fuzz harness for the lexer/parser.
- **Acceptance**: Parser recovers gracefully and produces stable diagnostics in the UI test harness.

4. Semantic Core: AST → Name Resolution → Type & Effect Inference — *depends on Step 3*
- **Name resolver**: Two-pass name resolution with DefIds and module scoping.
- **Type system (bidirectional)**: Implement a bidirectional type/inference pass with basic generics and a first-class representation for effect sets.
- **Effect inference**: Infer built-in effects (`pure`, `io`, `async`, `panic`) transitively; require explicit effects for public API boundaries.
- **Acceptance**: Hello-world, simple generics, and IO-annotated functions compile and typecheck.

5. MIR Lowering, Drop Insertion & Borrow Checker (Polonius) — *depends on Step 4*
- **MIR design**: CFG-based mid-level IR with explicit ownership and drop sites.
- **Drop insertion & liveness**: Deterministic drop placement to keep semantics stable.
- **Polonius integration**: Adapt `polonius-engine` input format; support field projections, precise region generation.
- **Acceptance**: Borrow errors detected (use-after-move, conflicting borrows); field-level borrows validated.

6. Minimal Codegen & Runtime (dev fast path) — *depends on Step 5*
- **LIR + codegen**: Emit a simple LIR then Cranelift-backed codegen for dev builds; support basic primitives, function calls, control flow.
- **Runtime**: Minimal runtime (stack, heap allocator shim) sufficient to run tests and bootstrapping tasks.
- **Acceptance**: `omni run hello.omni` executes native hello-world produced by Stage0-built toolchain.

7. Standard Library Strategy & Preservation — *parallel with Step 5/6*
- **Preserve originals**: Save full stdlib sources as preserved copies (e.g., `core.orig.omni`) and replace active files with small parseable stubs exposing only needed signatures and types (follow your preservation policy).
- **Iterative re-enable**: Reintroduce smallest, lowest-dependency functions from preserved originals, run Stage0 `--emit-ast`/typecheck after each re-enable.

Recent re-enables (Stage0-safe builtins):

- Added `str_len` and `string_concat` as Stage0-safe builtins (type signatures + interpreter implementations) to accelerate iterative re-enablement and example validation.
- Added `string_eq` and `string_push_char` as builtins to cover common string operations during bootstrap.

Workflow note: builtins are provided in the Stage0 interpreter and declared in the type-checker to allow gradual in-repo restoration of real implementations from `core.orig.omni` without breaking Stage0 checks.
- **Acceptance**: Core surface types (`Option`, `Result`, minimal `String`, basic `Vec`) available and testable.

8. Tests, Fuzzing, and Diagnostics — *ongoing throughout prior steps*
- **UI tests**: parser/diagnostic UI tests, bidirectional type tests.
- **Fuzzing**: `cargo-fuzz` harnesses for lexer/parser/serialization.
- **Diagnostic quality**: Elm/Rust-style multi-span diagnostics with machine-applicable fixes for trivial fixes.
- **Acceptance**: CI runs UI and fuzz targets; diagnostics include stable codes and fix suggestions.

9. Tooling: LSP & Language Server — *depends on Steps 4–6; do not ship fully until core stable*
- **Query-based server**: Implement an LSP backed by a Salsa-like incremental query model for fast responses.
- **Features**: hover with effect sets, inlay type/effect hints, go-to-def, borrow-visualization (post-MIR).
- **Acceptance**: LSP provides correct go-to-def and hover types for standard library and small projects.

10. Advanced Type & Effect Features — *depends on Steps 4–6*
- **Generics & implied bounds**, trait system, async as effect, effect polymorphism, negative bounds, variadic generics.
- **Macros & comptime**: Declarative macros + sandboxed procedural macros + `comptime` features.
- **Acceptance**: Library-level features compile and tests validate correctness.

11. Optimizations & Backends — *depends on Step 6*
- **Optimizing MIR passes**: inlining, dead-code elimination, simple constant-folding.
- **Secondary backends**: LLVM/inkwell for release builds; MLIR pipeline for tensor acceleration (Phase later).
- **Acceptance**: Release builds with LLVM pass performance tests and MLIR integration validated on small tensor workload.

12. Self-hosting Migration (Stageed) — *depends on full frontend+MIR+codegen+stdlib*
- **Stage1**: Use Rust Stage0 to build a minimal Omni compiler binary (Stage1) compiling a conservative subset.
- **Stage2**: Use Stage1 to compile the Omni compiler sources to produce Stage2.
- **Parity verification**: Implement deterministic build process and reproducible binary comparison (Stage1==Stage2). If mismatch, iterate until identical.
- **Acceptance**: Binary parity verified by deterministic build (bitwise or reproducible metadata-stripped hash equality) and Stage2 can compile the whole stdlib and itself.

13. Platform & Release — *post self-hosting*
- **Multi-platform CI**: Linux/macOS/Windows runners with reproducible-build guards.
- **Packaging**: installers, `omni` CLI distribution, registry publishing with API compatibility checks.
- **Acceptance**: Release pipeline produces signed artifacts and reproducible builds across platforms.

**Relevant files / artifacts (reference)**
- **Specification**: the project spec (Omni_Complete_Specification.md) — primary design source.
- **Planned crates**: Stage0 CLI crate, compiler library crate, stdlib repo (preserved originals + active stubs), LSP crate, tools/test crates — create under a workspace.
- **Test harnesses**: UI test directory (parser/ui), fuzz harness, integration tests and small programs for bootstrap verification.

**Verification**
1. **Unit & UI tests**: Add unit tests per compiler pass; UI tests for diagnostics.
2. **Fuzzing**: Run `cargo-fuzz` targets for lexer/parser continuously in CI.
3. **Bootstrap parity check** (core acceptance):
- Build Stage0 (Rust) producing `omni-stage0`.
- Use `omni-stage0` to build the Omni compiler sources producing `stage1`.
- Use `stage1` to build the Omni compiler sources producing `stage2`.
- Compare `sha256sum stage2_built_by_stage0` and `sha256sum stage2_built_by_stage1` — require equality.
4. **Integration tests**: run a curated suite of small programs (hello, fizzbuzz, concurrency examples) under each Stage to validate runtime semantics.

**Decisions (from your responses)**
- **Bootstrap**: Use Rust Stage0 as the trusted baseline (confirmed).
- **Repo action**: I will scaffold a minimal Rust Stage0 workspace now (you selected scaffold).
- **Preservation policy**: Preserve originals as `.orig.omni` and implement minimal parseable stubs (confirmed).
- **Priority**: Follow vertical-slice approach: finish the core compiler pipeline before enabling/expanding LSP/HELIOS.
- **Platforms**: Target Linux, macOS, and Windows in CI initially.
- **Self-hosting criterion**: Stage1==Stage2 binary parity and self-compilation end-to-end (confirmed).

**Further Considerations**
1. **Incremental reconciliation**: Re-enable preserved stdlib parts one small API at a time; add tests that exercise each newly enabled function before proceeding.
2. **Dependency choices**: Use `rowan`-like CST, `salsa`-style query engine for incremental builds, `polonius-engine` for borrow checking, `cranelift` for dev codegen, `LLVM/inkwell` for release; keep these dependencies in Stage0 only while avoiding reliance on external language runtimes.
3. **Determinism**: Design the build pipeline for deterministic outputs early (strip timestamps, embed reproducible metadata) — this reduces bootstrap debugging cost.
4. **Resource plan**: Self-hosting parity engineering is time-consuming; expect the majority of effort in Steps 4–6 (semantic core and borrow checker) and Step 12 (parity verification).
5. **Next immediate action**: scaffold the Stage0 Cargo workspace and minimal stdlib stubs (as you approved). I will produce a concrete scaffold checklist (crates to create, manifest templates, CI skeleton, preservation plan) as the next deliverable.

