# PR Notes: Incremental stdlib re-enable (Step 7)

Summary
- This PR incrementally re-enables a small, safe surface of the Omni stdlib
  to unblock Stage0 parsing and typechecking while preserving originals.

Files changed
- `omni/stdlib/core.omni` — added Option/Result/String/Vector/HashMap/HashSet
  parseable stubs and small helper signatures (non-destructive, minimal bodies).
- `scripts/preserve_stdlib.ps1` — helper to preserve originals and generate
  stub templates under `omni/stdlib/stubs/`.
- `docs/stdlib_reenable.md` — changelog and next steps.
- `crates/lir`, `crates/codegen-cranelift` — LIR scaffolding and interpreter/JIT
  stub (Step 6 completed).

How to review
- Inspect `omni/stdlib/core.omni` for the added declarations and confirm they
  match the expected API surface to reintroduce.

Local verification
1. Run Stage0 checks (parse/type/emit-mir/run-mir) on examples:

```bash
cargo run -p omni-stage0 -- parse examples/hello.omni
cargo run -p omni-stage0 -- check examples/hello.omni
cargo run -p omni-stage0 -- emit-mir examples/hello.omni
cargo run -p omni-stage0 -- run-mir examples/hello.omni
```

2. Run unit tests:

```bash
cargo test --workspace
```

Notes & Next Steps
- Re-enable more stdlib items gradually: string API, more collection helpers,
  minimal runtime bindings under `crates/omni-stdlib` where tests require them.
- Consider gating larger re-enables behind a feature or incremental PRs to keep
  review manageable.
