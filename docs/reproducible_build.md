# Reproducible Build Guide

This repository currently verifies reproducible `omni-stage0` release builds on
Windows by using a fixed target directory and stable linker settings.

## Recommended workflow

1. Use a fixed target directory for the comparison run.

```powershell
$env:SOURCE_DATE_EPOCH = "1600000000"
$env:RUSTFLAGS = "-C debuginfo=0 -C link-arg=/Brepro"
$env:CARGO_TARGET_DIR = "target/repro"
```

2. Build the Stage0 release binary twice, cleaning between runs.

```powershell
cargo clean
cargo build -p omni-stage0 --release
cargo clean
cargo build -p omni-stage0 --release
```

3. Compare the resulting binaries.

```powershell
Get-FileHash target/repro/release/omni-stage0.exe -Algorithm SHA256
```

## Notes

- The current Windows binary embeds build-path metadata when different target
  directories are used, so comparisons across `target\parity1` and
  `target\parity2` do not match byte-for-byte.
- The helper script `scripts/compare_reproducible_build.ps1` can inspect two
  binaries and normalize the build-path fragments when you need to understand
  the remaining drift.
- For the current bootstrap workflow, the stable-path check above is the one
  used to validate reproducibility.