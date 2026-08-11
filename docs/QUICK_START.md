# Omni v0.1.4.1.1 quick start

```bash
cargo build --release --locked -p omni-stage0
cargo install --path crates/omni-stage0 --locked --force
omni --version
omni doctor
```

Expected identity: `omni 0.1.4.1` on the qualified x86-64 Linux/WSL2 host.

Run the scalar smoke and all cumulative conformance:

```bash
omni check examples/native_edition1.omni
omni run examples/native_edition1.omni
python3 scripts/historical-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_layout_v0_1_3/manifest.json
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_value_abi_v0_1_4/manifest.json
```

v0.1.4.1.1 adds bounded aggregate arguments/returns, immutable String and binary Bytes values,
primitive byte values and the bootstrap allocator/scalar-cell collection foundation. It
does not yet qualify ownership-sensitive mutation, escaping stack slices or source-level
generic mutable collections.

See `MILESTONE_0.1.4.1_STRING_BYTE_VALUE_ABI_COLLECTIONS_FOUNDATION.md` and
`CURRENT_IMPLEMENTATION_MATRIX.md` for the exact boundary.
