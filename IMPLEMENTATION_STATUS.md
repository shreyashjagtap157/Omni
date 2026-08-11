# Omni implementation status

The authoritative detailed status is `docs/CURRENT_IMPLEMENTATION_MATRIX.md`.

Current milestone: **v0.1.4.1.1 — String/Byte/Value ABI + Collections Foundation**.

The cumulative v0.0.1 through v0.1.3 scalar/control-flow/local-layout baseline remains
intact. v0.1.4.1.1 adds typed function ABI metadata, bounded indirect aggregate arguments,
caller-owned aggregate returns, immutable two-cell String/Bytes descriptors, runtime
String/Bytes printing, primitive byte values, binary-safe literals and bounds-checked byte
indexing. `crates/omni-stdlib` now also provides an explicit bootstrap `CellAllocator`
contract and failure-atomic checked scalar-cell vector foundation.

This milestone deliberately does **not** claim production ownership/borrowing, nontrivial
destruction, ownership-sensitive aggregate mutation, mutable heap String/Bytes semantics,
source-level generic mutable collections, escaping-slice ABI, full generic/trait-native
execution, async, stable FFI, or multiple canonical native ISAs. Those paths remain fail
closed.

Promotion requires source audits, workspace fmt/Clippy/tests/build, historical 5-case
compatibility, the inherited 23-case scalar corpus, inherited 10-case v0.1.3 layout corpus,
the 10-case v0.1.4.1.1 value-ABI corpus, and the cumulative 60-second deterministic parser/
lexer fuzz gate on the pinned Rust 1.97.1 x86-64 Linux/WSL2 host.
