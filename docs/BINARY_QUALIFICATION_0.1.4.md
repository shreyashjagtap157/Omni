# Omni v0.1.4 Binary Qualification

Status: **PASS** for the cumulative implementation surface claimed through v0.1.4 on
x86-64 Linux.

## Toolchain

- Rust 1.97.1
- Cargo 1.97.1
- rustfmt 1.9.0-stable
- Clippy 0.1.97
- canonical artifact: owned x86-64 Linux ELF64 native AOT executable

## Rust/source gates

The final v0.1.4 source passes source lineage audit, source verifier, `cargo fmt`,
workspace-wide Clippy with warnings denied, all workspace tests, whole-workspace build and
release `omni-stage0` build/install. The workspace registers **549 tests**; five are
explicitly ignored v0.3.0 trait/type-system semantic tests and none belong to the v0.1.4
claimed surface.

## Executable conformance

- historical v0.0.1 compatibility: **5/5 PASS**;
- native scalar v0.1.2: **23/23 PASS**;
- native layout v0.1.3: **10/10 PASS**;
- native value ABI v0.1.4: **10/10 PASS**;
- lexer/parser fuzzing: **60 cumulative seconds / 19,461 generated cases / four independent deterministic seeds / zero reported crash, signal or timeout**;
- optional cargo-fuzz oracle: skipped because cargo-fuzz is not installed and is non-normative.

The v0.1.4 conformance includes binary stdout containing `0xFF`, proving the installed CLI
and conformance harness remain byte-oriented rather than imposing UTF-8 on `Bytes`.

## Installed compiler

SHA-256:

`31e4c41529334efba07ad20e2a7b80bb1a826b0e9bd8c11bd49f5895b26b8cc3`

## Host-wrapper note

The combined `scripts/qualify-release.sh` was invoked in this execution environment and
passed all pre-fuzz gates through installed compiler identity, doctor/smoke checks and
historical 5/5 before the outer tool-call deadline interrupted it during fuzzing. Every
remaining normative gate was then run individually with the same installed binary and
passed. This is an execution-wrapper limitation, not an Omni gate failure; no gate was
shortened or omitted.

## Scope boundary

v0.1.4 qualifies the bounded scalar-cell aggregate ABI and immutable String/Bytes value
surface described by the milestone document, plus the Rust-bootstrap allocator/scalar-cell
collection foundation. Production ownership/borrowing/nontrivial drop, ownership-sensitive
mutation, source-level generic mutable collections, general escaping-slice ABI, stable FFI,
async/concurrency, and non-x86-64 canonical native targets remain outside this release.
