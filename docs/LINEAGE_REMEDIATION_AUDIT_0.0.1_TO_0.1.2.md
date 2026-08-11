# Omni lineage remediation audit — v0.0.1 through v0.1.2

**Audit date:** 2026-08-08  
**Scope:** original Stage-0 v0.0.1 contract, evolved local implementation, v0.1.1 native-core milestone, v0.1.2 scalar-control-flow milestone, all live source/tooling boundaries that can affect those claims.  
**Result:** **no known unresolved source-level blocker against the declared v0.0.1–v0.1.2 milestone surface after remediation.** Full promotion is still blocked on Rust-host compilation/test/native execution because the packaging sandbox has no Rust toolchain.

## Why this audit was necessary

The uploaded repository had evolved far beyond the tiny original bootstrap. It contained
large frontend, tooling and backend experiments, historical successful build logs, and
plans that described several different source states. That created three dangerous
failure modes:

1. historical green logs could be mistaken for proof that the exact current checkout builds;
2. parser/AST scaffolding could be mistaken for complete language semantics;
3. optional/mock backends could silently substitute for the native execution contract.

The remediation rule is now simple:

> Anything claimed by v0.0.1 through v0.1.2 must work end-to-end or fail explicitly.
> Future features may remain in source, but they cannot fabricate success.

## Historical milestone closure

### v0.0.1

The original Stage-0 contract required a tiny Edition-1-like source subset, checked
`i64` add/subtract/multiply, local definitions, literal printing, return values, direct
x86-64 Linux ELF generation, and no mandatory VM/JIT/runtime dependency in the emitted
program.

That contract is preserved as an explicit five-case compatibility suite at
`conformance/historical_v0_0_1/`. The modern compiler is a source-language superset,
but the old semantic wedge remains a compatibility requirement.

### v0.1.0

The uploaded v0.1.0-era tree is treated as a **broad experimental snapshot**, not a
stable language promise. Its useful source was retained, but incomplete advanced paths
were either repaired, made fail-closed, or archived. Historical logs remain evidence of
past progress only.

### v0.1.1

The native-core baseline was repaired around the owned `codegen-native` path. Direct
function parameters, checked scalar arithmetic, optimizer legality, LIR verification,
SysV call alignment, direct ELF emission and backend independence are retained.

### v0.1.2

Scalar control flow now includes `if`, `while`, `loop`, cleanup-aware `break` and
`continue`, MIR branch-target verification, project-directory workflows, and a ten-case
native conformance suite. Project builds are intentionally restricted to dependency-free
single-entry projects; external dependency resolution and `build.omni` scripts fail
closed until their security/reproducibility semantics are actually qualified.

## Major defects repaired

The machine-readable registry at `release/LINEAGE_REMEDIATION_ISSUES.json` records the
full set (**46 fixed findings**). Major classes include:

- missing Rust modules in the uploaded source snapshot;
- canonical “native” execution accidentally routed through Cranelift;
- default heavyweight/JIT/backend dependencies;
- LLVM backend-name aliasing/fallback;
- optimizer removal of effectful calls;
- arithmetic constant-folding faults and host-overflow behavior;
- unsupported constructs degrading to fake values or no-ops;
- unresolved variable/label fallbacks;
- phantom return-value ABI behavior;
- hard-coded direct-call behavior and SysV stack-alignment problems;
- FFI simulated success;
- compile-time and macro-time invented Unit/unchecked arithmetic;
- incorrect trait implementation registration;
- invented package versions;
- placeholder CLI/docs behavior;
- mock/experimental borrow checking appearing canonical;
- raw generational arena lifetime/leak hazards;
- duplicate arena release accounting;
- interpreter negative-index/arithmetic/missing-value fallbacks;
- malformed LSP positions defaulting to valid coordinates;
- stale build-graph metadata being trusted after filesystem errors;
- unhermetic `build.omni` execution;
- formatter read failures defaulting to empty content;
- duplicate/partial stdlib source surfaces;
- misleading public examples;
- self-host smoke test using an incompatible legacy source file;
- stale contributor/agent plans reintroducing obsolete architecture assumptions;
- release qualification incorrectly using all experimental workspace crates.
- stale installation text contradicting the fail-closed build-script boundary;
- floating bootstrap toolchain version.

## Source-of-truth cleanup

Historical plans, audits, stdlib prototypes, backend implementations and parser examples
are preserved under `docs/archive/`. They are not deleted because they remain useful
for later differential work and design history, but they no longer compete with current
status documents.

Current authority order:

1. `spec/README.md` + `spec/edition1/` — semantic target;
2. `docs/CURRENT_IMPLEMENTATION_MATRIX.md` — implementation coverage;
3. `docs/VERSIONING_AND_BOOTSTRAP_PLAN.md` — future sequence;
4. this audit + `release/LINEAGE_REMEDIATION_ISSUES.json` — historical closure;
5. executable qualification gates — evidence.

## Current explicit non-claims

The remediation does **not** pretend to complete future roadmap work. v0.1.2 does not
claim production ownership/borrowing, aggregates/layout, full strings/collections,
floating point, generics/traits native execution, effects/capability runtime, async,
concurrency/atomics memory-model conformance, stable FFI ABI, multiple native ISAs,
managed memory, accelerators, persistence, distribution, complete standard library, or
self-hosting.

Those are not baseline defects while they are absent from the milestone contract and
fail closed when reached.

## Automated offline evidence

`python3 scripts/audit-baseline.py` checks:

- all workspace version identities;
- exact default dependency closure;
- absence of heavyweight/network dependencies in that closure;
- lockfile/workspace consistency;
- historical v0.0.1 corpus integrity;
- current v0.1.2 corpus integrity;
- explicit fail-closed experimental backends;
- native-backend dependency boundary;
- absence of active Omni-source stdlib prototypes;
- fuzz-source retention;
- absence of shipped Cargo `target/` trees;
- production TODO/unimplemented markers;
- structural delimiter sanity for active Rust source;
- selected regression patterns for previously discovered silent fallbacks;
- current documentation/backend qualification assumptions.

`python3 scripts/verify-source.py` independently checks packaging/version/module/source
invariants.

Both pass in the packaging environment after this remediation.

## What is still unverified here

The sandbox has no `cargo`, `rustc`, `rustfmt`, or Clippy, so this audit cannot honestly
assert that the modified Rust checkout compiles. The final local gate is:

```bash
./scripts/qualify-release.sh
```

on x86-64 Linux/WSL2 with the repository-pinned Rust 1.97.1 toolchain. It must pass formatting, Clippy, default-member
unit/integration tests, release compiler build/install, `omni doctor`, historical
compatibility, and current native conformance before this source candidate is promoted.

## Promotion decision

**Do not begin v0.1.3 implementation on the canonical branch until that Rust-host gate
passes.** If it fails, every failure is a v0.1.2 remediation defect and takes priority
over new language features.
