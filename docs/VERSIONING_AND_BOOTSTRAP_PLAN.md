# Omni implementation versioning and bootstrap plan

This roadmap separates **language capability** from **implementation language**.
Versions advance only when their exit gates are demonstrated by tests and native
artifacts.

## v3.4 reconciliation note

The repository's converged specification archive now lives at
`docs/archive/historical-plans/Omni_Complete_Specification_v3.4.md`. The current
bootstrap line still reports v0.x milestones for qualified native behavior, but the
implementation plan and status docs should be read against the v3.4 rule set when
describing semantic gaps or continuation work.

## Rust bootstrap: 0.0.1 -> 1.0.0 — complete dependable Omni Core

- **0.0.1** — minimal Rust bootstrap, lexer/parser/type-check, proof-of-concept native
  machine emission.
- **0.1.0** — broad experimental local compiler snapshot: MIR/LIR, multiple backend
  experiments, package/LSP/borrow scaffolding.
- **0.1.1** — native core baseline: repaired source integrity, owned x86-64 Linux ELF
  emitter, canonical native `run/build`, real scalar function calls, checked arithmetic,
  LIR stack/CFG validation, SysV call alignment, minimal default dependency closure,
  fail-closed unsupported semantics.
- **0.1.2** — native scalar correctness hardening: stable diagnostics, dependable
  `if`/`while`/`loop` control flow including `break`/`continue`, strengthened MIR/LIR
  verification, project-directory smoke workflows, and a source-to-native conformance corpus.
- **0.1.3** — native data layout I: arrays/slices/tuples/structs/enums and safe bounds,
  alignment, initialization, drop semantics.
- **0.1.4.1** — string/byte/value ABI and collections foundation with allocator interface.
- **0.2.0** — ownership/borrowing/regions semantics implemented end-to-end with a real
  production borrow engine and negative conformance corpus.
- **0.3.0** — generics, traits, associated items, coherence, specialization rules, and
  real monomorphization.
- **0.4.0** — effects/capabilities, Result/Option, contracts baseline, panic/error and
  cleanup semantics.
- **0.5.0** — structured concurrency, async lowering, atomics, channels, cancellation,
  and language memory-model implementation.
- **0.6.0** — portable optimizer baseline, translation validation hooks, PGO-ready IR,
  no semantic-changing release/debug modes.
- **0.7.0** — owned x86-64 PE/ELF and AArch64 object/executable paths; C ABI baseline.
- **0.8.0** — RV64 plus freestanding AArch64/RV64 images; MMIO/interrupt/DMA/bare-metal
  core facilities.
- **0.9.0** — package/build/reproducibility/security stabilization; debugger/LSP/formatter
  and conformance integration; feature freeze.
- **1.0.0** — complete stable Omni Core: self-consistent language semantics for the
  required core profiles, native Tier-1 implementations, stable core ABI, conformance
  corpus, reproducible Rust-bootstrap release.

`1.0.0` does **not** mean every optional advanced profile is complete. It means the
core is dependable enough that later facilities extend it instead of redefining it.

## Rust bootstrap: 1.0.1 -> 2.0.0 — complete specified Omni platform/language

This line completes the broader Edition-1 profile suite while preserving 1.x core
semantics:

- managed domains and collectors;
- full async/runtime providers;
- SIMD/scalable vectors, matrices/tensors, GPU/NPU target paths;
- persistent/storage profile including advanced I/O, NVMe/ZNS adapters where supported;
- distributed profile and explicit partial-failure semantics;
- realtime and constant-time assurance profiles;
- advanced optimizer, LTO/PGO/post-link optimization and translation validation;
- full package registry, signing, provenance, update and SBOM tooling;
- mature debugger/profiler/trace/LSP/docs/test/fuzz/benchmark suite;
- formal semantic models and proof-kernel integration required by the specification;
- all standard-library/profile contracts intended for the complete Edition-1 product.

**2.0.0** is the last release for which Rust is permitted to be the canonical compiler
implementation dependency.

## Self-host transition: 2.0.1 -> 3.0.0

The self-hosting transition is incremental and cross-validated rather than a rewrite
performed for ideological purity.

- **2.0.1–2.2.x** — compiler core libraries and source infrastructure ported to Omni.
- **2.3.x** — parser, resolver, type/effect system, ownership checker and MIR builder
  compiled by the Rust reference compiler.
- **2.4.x** — optimizer, machine descriptions and owned native backend ported.
- **2.5.x** — assembler/linker/object writers, package/build tooling and standard tools
  ported.
- **2.6.0** — Omni compiler successfully compiles its own compiler.
- **2.7.0** — deterministic stage convergence plus differential agreement against the
  Rust implementation.
- **2.8.0** — canonical clean builds stop invoking Cargo/rustc; Rust compiler becomes a
  diverse verification implementation only.
- **2.9.x** — bootstrap seed minimization, diverse double compilation, reproducibility,
  target and ecosystem hardening.
- **3.0.0** — fully self-hosted canonical Omni toolchain with an auditable bootstrap
  route and no required general-purpose foreign-language compiler in normal builds.

The Rust implementation should remain archived and buildable as a diversity oracle.
Removing independent implementations would make compiler-trust verification worse,
not purer.
