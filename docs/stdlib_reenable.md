# Stdlib Incremental Re-enable Log

This file records the conservative, incremental re-enablement of the Omni standard
library active sources from preserved originals. Follow the policy: never delete
originals — preserve them as `.orig.omni` and reintroduce the smallest, lowest-
dependency items first.

Completed steps
- Created preservation helper `scripts/preserve_stdlib.ps1` to copy `.omni` files
  to `.orig.omni` and generate stub templates in `omni/stdlib/stubs/`.
- Added `omni/stdlib/core.omni` minimal surface and reintroduced a set of small
  parseable stubs: `Option`, `Result`, `String`, `Iterator` trait, `panic` shim,
  `option_is_some`, `option_unwrap_or`, `result_is_ok`, `result_unwrap_or`, and
  `str_len`.
- Ran Stage0 checks (`parse`, `check`, `emit-mir`, `run-mir`) on examples and
  verified no regressions; Polonius mock reported the expected use-after-move in
  `examples/move_error.omni`.

Next actions (recommended order)
1. Identify and re-enable small collection types used by the compiler tests
   (e.g., lightweight `Vec<T>` surface signatures) and any trait methods the
   compiler expects at parse/type level.
2. Re-enable `String` API surface (length, concat) minimal stubs with parseable
   signatures.
3. Add unit tests that exercise each newly re-enabled symbol and run Stage0
   after each change.
4. If a re-enabled symbol requires runtime behavior for tests, provide a
   minimal Rust runtime implementation under `crates/omni-stdlib` and link to it
   in CI tests.

Notes
- Re-enable only signatures and small, dependency-free helpers first. Avoid
  reintroducing large implementations until the compiler and bootstrap pass are
  stable.
