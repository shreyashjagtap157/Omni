# ADR-0002: Workspace Structure and Crate Organization

Status: Accepted, amended by v0.1.2-r2 and v0.1.3 qualification

Original date: 2026-05-17  
Amended: 2026-08-08

## Context

Omni is a multi-crate Rust bootstrap compiler. The workspace must separate compiler
stages and experimental backends without allowing experiments to enter the canonical
native semantics or inflate the default build dependency closure.

## Decision

The workspace may contain experimental/future crates, but the **qualified default
members** for the current v0.1.4 semantic closure are:

- `omni-stage0` — bootstrap CLI;
- `omni-compiler` — frontend, semantic passes and MIR;
- `lir` — target-neutral low-level IR;
- `codegen-native` — owned x86-64 Linux AOT/ELF backend;
- `omni-stdlib` — Rust bootstrap helper structures.

The canonical dependency direction is:

```text
omni-stage0 -> omni-compiler -> codegen-native -> lir
                           -> omni-stdlib
```

`codegen-cranelift`, `codegen-llvm`, `codegen-mlir`, `codegen-wasm`, Polonius research,
self-host scaffolding, release experiments and fuzz crates may remain in the repository,
but they are not part of the v0.1.4 semantic qualification closure unless a later
milestone explicitly re-qualifies them.

### Backend rule

- Owned native AOT is canonical.
- Cranelift/LLVM compatibility entry points fail closed in v0.1.4.
- No backend name may silently substitute a different backend.
- LLVM/Cranelift SDK/runtime dependencies are not required by the default compiler.

### Ownership rule

The former mock/upstream Polonius paths are **not** canonical borrow-soundness proofs.
Ownership-sensitive MIR fails closed until the ownership milestone provides a real,
versioned, end-to-end checker and conformance corpus.

### Fuzzing rule

Fuzz target source stays in version control. Generated fuzz targets, corpora, crashes,
coverage, and Cargo build output are disposable and excluded from release archives.

## Consequences

- Default builds are substantially smaller and do not compile heavyweight backend SDKs.
- Experimental source remains available for later differential work.
- The canonical semantic claim remains the qualified default closure, while v0.1.4 also
  requires the retained workspace scaffolds to compile, pass warning-denied Clippy, and
  pass their workspace tests so archived/future crates cannot accumulate hidden Rust-level
  breakage.
- Future ADR amendments must update the implementation matrix and qualification scripts.
