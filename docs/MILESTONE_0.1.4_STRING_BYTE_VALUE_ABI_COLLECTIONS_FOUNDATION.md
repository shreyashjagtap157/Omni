# Omni v0.1.4 — String/Byte/Value ABI + Collections Foundation

## Achievement

v0.1.4 extends the v0.1.3 local scalar-cell layout into a bounded cross-function value ABI
and establishes binary/string values plus an allocator-backed collection substrate without
prematurely claiming ownership semantics.

## Value ABI

MIR function signatures retain parameter and return type information. LIR classifies
values as unit, scalar, or bounded indirect storage. For the current owned x86-64 backend:

- scalar values continue through the existing integer register/stack call convention;
- aggregate parameters are passed as pointers carrying a statically validated cell span;
- aggregate returns use hidden caller-owned storage;
- a callee never returns a pointer into its local frame;
- forwarding an indirect parameter preserves its validated span;
- indirect offsets are checked against declared cell count and 8-byte alignment before emission.

The qualified aggregate ABI is intentionally limited to current scalar-cell structs/enums
and the two-cell String/Bytes descriptors. This is not yet the stable general Omni ABI.

## String

`String` is currently an immutable two-cell descriptor:

```text
cell 0: pointer to immutable UTF-8 bytes
cell 1: byte length
```

Qualified behavior includes literal construction, pass/return, `.len` as UTF-8 byte length,
and runtime printing from a descriptor. Direct `String[index]` is rejected because indexing
UTF-8 by byte, code point, scalar value or grapheme cluster must not be guessed.

## byte and Bytes

`byte` is a distinct 0-255 primitive value. `b'X'` and escaped byte literals are range
validated. `b"..."` is a binary `Bytes` value, not a String, and may contain arbitrary
non-UTF-8 octets via escapes such as `\xFF`.

`Bytes` uses the same two-cell `{data,len}` shape while retaining a distinct type. Qualified
behavior includes pass/return, binary-safe output and bounds-checked `Bytes[index]`. An
out-of-bounds access uses the established native bounds-fault exit status 102 before the
invalid load occurs.

The external conformance runner is byte-oriented and supports `stdout_hex`, ensuring the
test harness itself cannot accidentally reject valid non-UTF-8 output.

## Collections/allocator foundation

`crates/omni-stdlib` contains the Rust-bootstrap foundation:

- `CellAllocator` with allocate/grow/shrink/deallocate operations;
- `BootstrapCellAllocator`, a safe host-backed implementation used only by the bootstrap;
- `OmniCellVector`, a checked dynamic vector of raw 64-bit scalar cells;
- deterministic length/capacity and checked get/set/push/pop/reserve/shrink behavior;
- failure-atomic growth: if reserve/grow fails, length, capacity and prior elements remain unchanged.

This is deliberately a **foundation**, not a claim that source-level generic mutable
collections are complete. Exposing `Vector<T>` mutation before moves, borrows, aliases and
drop are sound would make the ownership milestone meaningless.

## Conformance

The cumulative CLI gate runs:

```bash
python3 scripts/historical-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_layout_v0_1_3/manifest.json
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_value_abi_v0_1_4/manifest.json
```

The v0.1.4 manifest contains 10 cases covering struct argument/return, enum ABI round-trip,
String round-trip/print, binary Bytes round-trip, checked byte indexing/OOB fault,
UTF-8 String-index refusal and ownership-sensitive mutation refusal.

## Deliberate boundary

v0.1.4 does **not** qualify production ownership/borrowing, nontrivial drops, aggregate
mutation, mutable/heap-owning String or Bytes, escaping stack slices, source-level generic
collections, general nested aggregate ABI, stable FFI, async/concurrency, or multiple
canonical ISAs. Those remain later semantic wedges and must fail closed when reached.
