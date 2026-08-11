# Omni v0.1.2 — Scalar Control-Flow Hardening

## Milestone purpose

v0.1.2 takes the v0.1.1 native core spine and closes correctness/usability gaps in the
scalar control-flow path. It is intentionally not a claim that the broad frontend is
fully executable. The rule remains: **implement semantics end-to-end or fail closed**.

## Achievement boundary

Canonical path:

```text
source -> static checks -> MIR -> MIR verifier -> LIR -> LIR verifier
       -> owned x86-64 machine code -> ELF64 -> Linux -> CPU
```

No VM, JIT, LLVM, C compiler, assembler or external linker is required to emit the
supported native artifact.

## New implementation in v0.1.2

### Loop control

- `break` and `continue` receive real MIR jump targets inside `loop` and `while`.
- nested loops receive independent lexical targets;
- `continue` in `while` jumps to condition re-evaluation;
- leaving loop scopes emits drops for locals in exited scopes;
- source-level `break`/`continue` outside a loop are hard errors (`4014`/`4015`);
- function bodies reset loop context, so a nested function cannot target its lexical
  parent's loop.

### MIR structural validation

A backend-independent verifier now rejects:

- duplicate MIR labels;
- unconditional jumps to missing labels;
- conditional jumps to missing labels.

This executes after MIR optimization and before backend lowering.

### Native-path verification retained/expanded

The owned native emitter verifies LIR before instruction emission, including:

- evaluation-stack underflow;
- inconsistent evaluation-stack depth across CFG merges;
- duplicate functions;
- invalid calls and branch targets;
- unsupported scalar ABI forms;
- unsupported memory operations.

SysV x86-64 call-site stack alignment is preserved for nested calls even while outer
expression values remain live.

### Project usability

- `omni new` now emits a brace/semicolon Edition-2026 project with a native-compilable
  `main`.
- `omni check <project-dir>` resolves `src/main.omni`.
- `omni run <project-dir>` uses native AOT.
- `omni build <project-dir>` loads `omni.toml`, writes `omni.lock`, and emits a native artifact for the qualified dependency-free project subset.
- `build.omni` is explicitly rejected in the remediated baseline until hermetic declared-input/output build actions are qualified.
- default project output is `target/omni/<package-name>`.

### Conformance corpus

`conformance/native_scalar_v0_1_2` contains 10 executable/static cases covering:

1. checked scalar arithmetic;
2. direct function calls;
3. nested-call ABI alignment;
4. `if`/`else`;
5. `while` + `break` + `continue`;
6. project-directory native run;
7. project-directory native build + external execution;
8. checked-overflow fault status;
9. illegal `break` diagnostic;
10. illegal `continue` diagnostic.

Run with:

```bash
python3 scripts/native-conformance.py --omni omni
```

## Correctness work inherited from v0.1.1

- calls are never discarded merely because their result is unused;
- faulting arithmetic is not constant-folded to different semantics;
- side-effectful functions are not constant-inlined;
- unresolved labels cannot silently become instruction zero;
- incomplete advanced constructs lower to explicit unsupported sentinels;
- direct function parameter flow reaches the native SysV ABI;
- unary negation operand order is correct.

## Deliberately unsupported at this milestone

Important Edition-1 features remain outside the dependable native subset, including
real aggregate layout, slices/strings as values, floating point, match, `for`, generic
monomorphization, production ownership/borrow soundness, effects/capabilities runtime,
async/concurrency, stable FFI ABI, managed memory, SIMD/tensors, multiple native target
formats and bare-metal images.

The next implementation milestone must not weaken this boundary by faking values for
those constructs.

## Lineage remediation

The post-milestone whole-lineage audit additionally removed mock ownership claims, disabled unqualified Cranelift/LLVM execution, hardened legacy interpreter/comptime fallbacks, corrected project build-graph uncertainty, rejected unhermetic `build.omni`, cleaned stale examples/docs, and introduced `scripts/audit-baseline.py`. See `LINEAGE_REMEDIATION_AUDIT_0.0.1_TO_0.1.2.md`.

## Qualification status in the packaging environment

Verified here:

- manifests and lockfile parse;
- path dependencies and module declarations resolve;
- workspace versions are synchronized;
- generated Cargo `target/` trees are absent;
- fuzz target source is retained;
- owned native backend depends only on `lir`;
- optional heavyweight backends are outside the default compiler feature closure;
- 10-case conformance manifest and sources are structurally valid;
- modified Rust source delimiter structure is balanced;
- `python3 scripts/verify-source.py` passes.

Not possible in this sandbox because `cargo`/`rustc` are absent:

- `cargo fmt`;
- `cargo clippy`;
- `cargo test`;
- Rust compilation/linking of the bootstrap compiler;
- execution of this exact compiler binary.

Promotion therefore requires the supplied local qualification script on a Rust-equipped
x86-64 Linux/WSL2 host.
