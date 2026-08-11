# Omni v0.1.3 — Native Data Layout I

## Achievement

v0.1.3 is the first Omni milestone with qualified aggregate values on the canonical
owned x86-64 Linux AOT path. It extends the v0.1.2 scalar/control-flow baseline with a
small, deterministic **local scalar-cell layout** for arrays, slices, tuples, structs,
and enums, plus executable bounds/alignment/initialization checks.

This is deliberately **data layout I**, not the stable public value ABI. Aggregate
arguments/returns, strings/bytes/collections, and ownership-sensitive mutation remain
later milestones and fail closed.

## Qualified canonical subset

### Structs

- Struct types are nominal: equal field shapes do not make two declarations the same type.
- Struct literals validate field names, completeness and field types against the declaration.
- Physical local field order is declaration order, independent of literal spelling order.
- Qualified fields are currently scalar cells: integer/i64/isize and boolean values.
- Field reads lower to checked local byte offsets and execute natively.

### Tuples

- Tuple values use structural, declaration/index order.
- Qualified elements are scalar cells.
- Constant tuple indexing is validated during lowering and becomes a checked local offset.

### Fixed arrays

- Array literals preserve homogeneous element typing through the frontend and MIR.
- Local array elements occupy contiguous eight-byte scalar cells.
- Dynamic indexing emits a dedicated bounds-checked LIR operation.
- Negative or `index >= len` accesses fault through the native bounds-fault path with
  process status `102` **before** an out-of-range memory access.

### Local slices

- A qualified slice is a non-escaping local view over a qualified array/slice backing
  aggregate.
- v0.1.3 qualifies constant start/end range construction (`a..b` and inclusive `a...b`).
- Slice construction validates range ordering and source bounds before native emission.
- Indexing into the resulting view may be dynamic and uses the same runtime bounds check as arrays.
- Dynamic slice *range construction*, escaping slices, slice parameters/returns, and the
  stable `{data,len}` ABI are deliberately deferred.

### Enums and matching

- Enum types are nominal.
- Qualified constructors use `Enum::Variant(...)` and validate payload arity/types.
- Local native layout is an internal tag cell followed by enough scalar payload cells for
  the largest variant; inactive payload cells are initialized to zero.
- Fieldless and scalar-payload variants execute through tagged local native storage.
- Qualified match arms bind scalar payloads and require frontend exhaustiveness.
- The final variant arm may be selected by elimination only after the compiler proves all
  variants of the same safe enum are covered.
- This local representation is **not** the future stable `omni_v1` enum ABI.

## Safety properties qualified in this milestone

### Alignment

Every currently qualified aggregate cell is eight bytes and native `LoadOffset` /
`StoreOffset` validation rejects negative, misaligned, or frame-escaping byte offsets
before artifact emission. Stack frames remain 16-byte aligned by the existing native ABI.

### Initialization

- Struct/tuple/array construction stores every qualified field/element before reads.
- Enum construction initializes the tag, zeros all payload capacity, then stores the active
  variant payload.
- Struct literals with missing/unknown fields and enum constructors with wrong payload
  arity are rejected.

### Bounds

- Dynamic array/slice element loads check signed lower and upper bounds at runtime.
- Constant tuple indexes and slice range creation are checked before native emission.
- A zero-length indexed LIR target is rejected before emission.

### Drop semantics

The v0.1.3 aggregate subset can contain only trivially-droppable scalar cells. Therefore
aggregate scope cleanup has no runtime destructor action and cannot leak an owned resource.
Nontrivial aggregate members are not accepted by the qualified layout. Observable
destruction ordering becomes meaningful only when owned/nontrivial values are introduced;
production ownership/borrowing/destruction remains the v0.2.0 milestone.

## Deliberate fail-closed boundaries

v0.1.3 does **not** claim:

- aggregate value argument or return ABI — deferred to the v0.1.4 value-ABI work;
- String/byte/collection aggregate members or stable slice/enum ABI — v0.1.4;
- nested aggregate cells, floating-point aggregate cells, or generic aggregate layout;
- ownership-sensitive aggregate field mutation, borrow-derived places, or nontrivial drop — v0.2.0;
- dynamic slice range construction or escaping slice lifetimes;
- guarded/or-pattern/nested-payload enum native lowering beyond the explicitly qualified match subset;
- non-x86-64 canonical native aggregate emission.

Unsupported paths must either be rejected by the frontend or reach an explicit fail-closed
MIR/LIR/backend diagnostic; they must not fabricate a value or fall back to another runtime.

## Conformance evidence

New milestone corpus:

```bash
python3 scripts/native-conformance.py \
  --omni omni \
  --manifest conformance/native_layout_v0_1_3/manifest.json
```

The v0.1.3 corpus contains 10 CLI-level cases and passes 10/10 on the qualification host.
The dedicated Rust integration file `aggregate_native_v0_1_3.rs` contains 18 positive and
negative source-to-LIR/native/boundary tests. The owned native backend has 15 direct tests,
including alignment, frame escape, zero-length indexing, valid indexed loads and OOB faults.

Historical v0.0.1 and v0.1.2 scalar conformance remain mandatory; v0.1.3 is cumulative.

## Promotion gate

Promotion requires the pinned Rust 1.97.1 x86-64 Linux/WSL2 environment to pass:

```bash
python3 scripts/audit-baseline.py --worktree
python3 scripts/verify-source.py --worktree
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo build --release --locked -p omni-stage0
python3 scripts/historical-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_layout_v0_1_3/manifest.json
```

The release qualification script remains the authoritative single command and also retains
the lexer/parser fuzz gates inherited from v0.1.2.
