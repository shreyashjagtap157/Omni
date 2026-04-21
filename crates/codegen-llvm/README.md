codegen-llvm optional LLVM backend

This crate contains a feature-gated LLVM backend scaffold.

Features
- `with_inkwell`: Pulls in the `inkwell` crate with the LLVM 14 backend feature enabled. Requires a compatible system LLVM installation and will compile native bindings (`llvm_sys`).
- `real_llvm`: Logical feature that enables the real LLVM backend code paths inside this crate. This feature does NOT automatically pull in the native dependency.

The real backend currently lowers LIR through an LLVM dispatch loop that handles control flow, calls, and return-buffer plumbing. The default `use_llvm` path still bridges to Cranelift unless `real_llvm` and `with_inkwell` are both enabled.

Building
- Build without requiring a system LLVM (compile-time fallback available):

  cargo check -p codegen-llvm --features real_llvm

- Build and run tests with a system LLVM (example; you must install LLVM first):

  # Linux / macOS (example for LLVM 14)
  export LLVM_SYS_140_PREFIX=/usr
  cargo test -p codegen-llvm --features with_inkwell,real_llvm

  # Windows PowerShell (example)
  $env:LLVM_SYS_140_PREFIX = 'C:\Program Files\LLVM'
  cargo test -p codegen-llvm --features with_inkwell,real_llvm

Notes & Troubleshooting
- If you see errors originating from `llvm_sys` (unresolved symbols / missing headers), ensure:
  - A compatible LLVM version is installed (check the `llvm-sys` version required by `inkwell` via `cargo tree -p inkwell`).
  - The corresponding `LLVM_SYS_<MAJOR>_PREFIX` environment variable points to the LLVM installation prefix (contains `bin`, `include`, `lib`).
- On Debian/Ubuntu: `sudo apt-get install llvm-dev libclang-dev clang`
- On macOS: `brew install llvm` and add the brewed llvm `bin` to PATH or set `LLVM_SYS_<MAJOR>_PREFIX`.
- On Windows: install the official LLVM release and set `LLVM_SYS_<MAJOR>_PREFIX` to the install folder.

CI Recommendation
- Gate `with_inkwell` builds on runners that have LLVM preinstalled.
- Keep `real_llvm` separate from pulling in native deps to avoid breaking developer builds by default.

If you want, I can attempt to detect a local LLVM installation now and try a `with_inkwell,real_llvm` build (it may require installing LLVM).

Helper scripts
- `scripts/setup-llvm.sh` — installs LLVM-14 (Debian/Ubuntu example) and prints the `LLVM_SYS_*` export to use.
- `scripts/run-llvm-tests.sh` — detects `llvm-config`, exports the appropriate `LLVM_SYS_*` variable for the current shell, and runs the codegen-llvm tests with `with_inkwell,real_llvm`.
- `scripts/setup-llvm.ps1` — PowerShell helper for Windows (attempts `choco install llvm` and sets session env var).

CI workflow
- Added `.github/workflows/llvm-backend.yml` — manual/PR-triggered workflow that installs system LLVM on runners and runs `cargo test -p codegen-llvm --features with_inkwell,real_llvm`.