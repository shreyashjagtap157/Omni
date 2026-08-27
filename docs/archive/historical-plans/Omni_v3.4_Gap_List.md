# Omni v3.4 Gap List

This file captures the current reconciliation gap list between the converged v3.4
specification archive and the current qualified Rust bootstrap implementation.

## Gap 1 - source-order observability traceability

Spec expectation:

* source semantics are observable in source order unless observational equivalence is
  proven.

Current state:

* the codebase documents fail-closed semantics and source-order sensitivity in several
  places;
* the implementation matrix does not yet record a complete per-pass proof trail for
  optimizer and lowering behavior.

Required follow-up:

* add explicit traceability across parser -> MIR -> LIR -> native lowering;
* add negative tests for reordering attempts;
* ensure docs state that reproducibility does not authorize semantic reordering.

## Gap 2 - reproducibility envelope separation

Spec expectation:

* reproducibility governs artifacts and metadata, not program meaning.

Current state:

* release docs mention deterministic and reproducible behavior;
* there is no matrix row or conformance corpus specifically proving that canonicalization
  does not rewrite evaluation order or control flow.

Required follow-up:

* document artifact canonicalization boundaries;
* add targeted tests or conformance cases that exercise canonical metadata handling;
* keep semantic order separate from packaging order.

## Gap 3 - ABI/FFI evaluation order

Spec expectation:

* arguments are evaluated in source order and then packed into ABI-defined locations.

Current state:

* the current value-ABI milestone covers bounded indirect aggregates and native x86-64
  lowering;
* the repository still lacks an explicit v3.4 traceability row for FFI lowering order.

Required follow-up:

* add explicit order-preservation tests;
* cover rejected reordering and canonicalization scenarios;
* separate ABI packing from evaluation semantics in docs.

## Gap 4 - provenance-preserving lowering

Spec expectation:

* provenance and allocation/subobject identity are semantic obligations.

Current state:

* provenance language already exists in the codebase and spec;
* the implementation matrix does not yet expose a pass-by-pass provenance audit.

Required follow-up:

* inventory provenance-sensitive passes;
* state which passes preserve, narrow, or reject provenance-sensitive behavior;
* add documentation for any fail-closed boundaries.

## Gap 5 - freeze and qualification model

Spec expectation:

* `Defined`, `Formalized`, `Implemented`, `Qualified`, and `Frozen` are distinct.

Current state:

* milestone docs still dominate the repository status vocabulary;
* the current docs do not yet expose a single freeze-gating traceability summary.

Required follow-up:

* keep milestone status, qualification status, and freeze status separate;
* update the matrix and changelog when a rule reaches a new status;
* do not imply freeze from spec text alone.

## Gap 6 - unified v3.4 continuation index

Spec expectation:

* continuation work should be batched by dependency wedge.

Current state:

* the new v3.4 plan exists, but the live docs do not yet point to this file as the gap
  index.

Required follow-up:

* link this gap list from the matrix and roadmap as the current reconciliation index;
* use it as the batch selector for the next implementation pass.

