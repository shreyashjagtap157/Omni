# Omni v0.1.3 Binary Qualification

Status: **PASS** for the cumulative implementation surface claimed by versions 0.0.1 through 0.1.3 on x86-64 Linux.

## Environment

- Rust `1.97.1`
- Cargo `1.97.1`
- rustfmt `1.9.0-stable`
- Clippy `0.1.97`
- Canonical artifact: owned x86-64 Linux ELF64 native AOT executable
- Installed compiler SHA-256: `b59b55f5c94c276f1b1e784f7b8a06c5b8569613b2229329cdf35a33b88d7c49`

## Rust/source gates

The v0.1.3 tree passes source audit/verification, `cargo fmt --all -- --check`, warning-denied whole-workspace Clippy, whole-workspace tests, whole-workspace build, and release `omni-stage0` build/install. The workspace registers **548 tests**. Exactly **five** are intentionally ignored, all assigned to the v0.3.0 trait/type-system semantic milestone; no v0.1.3 layout test is ignored.

## Runtime conformance

- Historical v0.0.1 compatibility: **5/5 PASS**.
- Native scalar v0.1.2 cumulative corpus: **23/23 PASS**.
- Native data-layout v0.1.3 corpus: **10/10 PASS**.
- Lexer/parser black-box fuzzing: **60 cumulative seconds**, **19,438 generated cases**, four independent deterministic seeds, zero crashes/signals/timeouts.
- Optional cargo-fuzz: skipped because `cargo-fuzz` is not installed; it is not part of the normative duration-based release gate.

The execution host applies a per-command ceiling, so the 60-second fuzz obligation was executed as four independent 15-second shards (seeds 660100–660103), the same host-compatible method recorded for v0.1.2-r2.

## Qualified v0.1.3 addition

The new semantic wedge is local scalar-cell aggregate layout: nominal structs, structural tuples/fixed arrays, bounds-checked local slices, nominal tagged enums, deterministic initialization, checked local offsets/alignment, and trivial aggregate cleanup. Details are in `MILESTONE_0.1.3_NATIVE_DATA_LAYOUT_I.md`.

## Deliberate limits

This qualification does **not** claim stable aggregate/string/byte/collection value ABI (v0.1.4), aggregate cross-function passing/return, escaping slices, production ownership/nontrivial destruction (v0.2.0), complete trait semantics (v0.3.0), non-x86-64 canonical native targets, or qualified LLVM/Cranelift execution. These paths remain explicit fail-closed boundaries where applicable.

Machine-readable evidence is in `../release/BINARY_QUALIFICATION.json`.
