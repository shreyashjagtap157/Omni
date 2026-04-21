Omni — Minimal Stage0 scaffold

This repository contains a minimal scaffold for the Omni language bootstrap (Stage0).

Quick start (requires Rust toolchain):

```bash
cargo build --workspace
cargo run -p omni-stage0 -- parse
```

What's included:
- `crates/omni-stage0` — minimal Stage0 CLI skeleton
- `crates/omni-compiler` — minimal compiler library skeleton
- `omni/stdlib` — placeholder stdlib stubs (preserved originals go in `*.orig.omni`)
- `.github/workflows/ci.yml` — basic CI workflow

Documentation:
- Full specification and design rationale: [docs/Omni_Complete_Specification.md](docs/Omni_Complete_Specification.md)
- Contribution guidelines: [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)
- Code of Conduct: [docs/CODE_OF_CONDUCT.md](docs/CODE_OF_CONDUCT.md)
- Security policy: [docs/SECURITY.md](docs/SECURITY.md)
- Implementation status and logs: [docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md), [docs/execution_log.md](docs/execution_log.md)

Next steps:
- Implement lexer & layout engine in `crates/omni-compiler`
- Implement CST and parser
- Re-enable stdlib pieces incrementally from preserved originals
