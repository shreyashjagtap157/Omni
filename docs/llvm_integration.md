# LLVM Integration Guide

This guide explains how to verify the `real_llvm` backend locally and in CI.

Prerequisites
- A compatible LLVM 14.x installation (tested with 14.0.6) accessible on the host.

Windows
1. Install LLVM 14.0.6 via Chocolatey (requires admin) or use the helper script:

```powershell
# Option A: Chocolatey (admin)
choco install -y llvm --version=14.0.6

# Option B: Download + extract using the helper script (no admin required)
powershell -File scripts/download-llvm-win.ps1 -Versions @('14.0.6')

# If successful, set the environment variable for llvm-sys
$env:LLVM_SYS_140_PREFIX = 'C:\path\to\llvm-14.0.6'
```

Linux/macOS
1. Install a prebuilt LLVM 14 package for your distribution or build from source.
2. Set `LLVM_SYS_140_PREFIX` to the prefix directory where `bin/clang` and `lib/libLLVM.so`/`libLLVM.dylib` are located.

Verifying the build

Once `LLVM_SYS_140_PREFIX` is set in the environment for the current shell, run:

```bash
cargo test -p codegen-llvm --features real_llvm,with_inkwell
```

CI recommendation
- Add a matrix job that provisions LLVM 14 (or uses a runner image with LLVM preinstalled).
- Use the project's `scripts/setup-llvm.ps1` on Windows runners or `scripts/setup-llvm.sh` on Unix runners to provision and set `LLVM_SYS_140_PREFIX` before invoking `cargo test --features real_llvm,with_inkwell`.

Fallback for local development
- If you do not have LLVM available locally, use the `with_inkwell_stub` feature to exercise the `real_llvm` API without a system LLVM install:

```bash
cargo test -p codegen-llvm --features real_llvm,with_inkwell_stub
```

This stub delegates to the Cranelift backend and is intended for development and CI scenarios where a full LLVM toolchain is not available.

Example GitHub Actions job
--------------------------
Add a matrix job similar to the following to verify the `with_inkwell` path on Ubuntu and Windows runners:

```yaml
	llvm-verify:
		name: LLVM integration verification
		runs-on: ${{ matrix.os }}
		needs: build
		strategy:
			matrix:
				os: [ubuntu-latest, windows-latest]
		steps:
			- uses: actions/checkout@v4
			- name: Set up Rust
				uses: actions-rs/toolchain@v1
				with:
					toolchain: stable
					profile: minimal
			- name: Install LLVM on Ubuntu
				if: matrix.os == 'ubuntu-latest'
				run: |
					sudo apt-get update
					sudo apt-get install -y llvm-14 llvm-14-dev libclang-14-dev clang-14
					echo "LLVM_SYS_140_PREFIX=$(llvm-config --prefix)" >> $GITHUB_ENV
			- name: Install LLVM on Windows
				if: matrix.os == 'windows-latest'
				shell: pwsh
				run: |
					choco install -y llvm --version=14.0.6
					$prefix = 'C:\Program Files\LLVM'
					"LLVM_SYS_140_PREFIX=$prefix" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
			- name: Build and test codegen-llvm with inkwell
				run: |
					cargo test -p codegen-llvm --features real_llvm,with_inkwell --quiet
```

