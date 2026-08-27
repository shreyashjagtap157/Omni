# Omni v3.4 Implementation Plan

## Goal

Advance the repository from the current v0.x bootstrap line toward the converged v3.4
specification by batching compatible work items together, minimizing churn, and keeping
the codebase fail-closed wherever coverage is still missing.

## Planning principles

1. Do not implement isolated points one at a time if they share dependencies with a
   larger coherent wedge.
2. Preserve current qualified behavior until a later batch explicitly expands it.
3. Treat docs, tests, qualification corpora, and changelog entries as part of the same
   change set.
4. Prefer batches that can be validated end to end with the existing Rust bootstrap.

## Batch 0 - repository reconciliation and gap inventory

Dependencies:

* current implementation matrix;
* current spec snapshot;
* existing conformance corpora;
* status/audit documentation.

Work items:

* reconcile the v3.4 adjudication against the live implementation matrix;
* mark historical lines that are superseded but still useful;
* identify code paths that are explicitly fail-closed versus accidentally missing;
* enumerate parser, MIR, LIR, backend, stdlib, and packaging gaps by release wedge;
* identify which gaps are documentation-only and which require source changes.

Outputs:

* traceability table;
* ordered gap list;
* updated roadmap notes.

## Batch 1 - semantic core reconciliation

Dependencies:

* parser/source syntax;
* type and effect checker;
* source-order semantics;
* failure-closed error paths.

Work items:

* reconcile source-order observability rules with parser/type-check behavior;
* harden any places where the bootstrap currently accepts a stronger meaning than the
  spec allows;
* ensure unsupported constructs fail explicitly at the earliest safe point;
* align diagnostics with the spec language around defined/formalized/qualified/frozen.

## Batch 2 - provenance and lowering invariants

Dependencies:

* MIR;
* LIR;
* owned native codegen;
* raw-pointer and allocation-identity-sensitive logic.

Work items:

* preserve provenance-preserving lowering through all IR boundaries;
* ensure no transformation rewrites source meaning under the guise of canonicalization;
* validate raw exposure/reconstruction rules are explicit and fail-closed;
* ensure the native backend and any auxiliary lowering passes preserve observable order.

## Batch 3 - ABI and FFI batch

Dependencies:

* typed function signatures;
* current value ABI coverage;
* native backend argument/return lowering;
* existing ABI tests.

Work items:

* confirm FFI lowering preserves source evaluation order;
* keep canonicalization limited to packaging/metadata;
* extend ABI tests to cover both accepted and rejected combinations;
* add traceable negative tests for reordering and semantic rewriting attempts.

## Batch 4 - qualification corpus expansion

Dependencies:

* native conformance harness;
* historical compatibility cases;
* current matrix gaps;
* any newly reconciled spec rules.

Work items:

* add batched conformance cases for the new spec rules;
* preserve historical cases that still represent required compatibility;
* expand negative corpus coverage for fail-closed boundaries;
* keep the corpus grouped by semantic wedge rather than by implementation file.

## Batch 5 - documentation, changelog, and release hygiene

Dependencies:

* validated implementation changes;
* stable traceability matrix;
* test and conformance outputs.

Work items:

* update the implementation status matrix;
* update the changelog with actual behavior changes;
* keep the versioning/bootstrap plan in sync;
* archive obsolete plans without deleting useful history;
* record any remaining release blockers explicitly.

## Expected continuation order

1. Reconcile docs and status.
2. Confirm current implementation behavior against the v3.4 rules.
3. Batch compatible source changes.
4. Run the strongest relevant tests for the batch.
5. Fix failures.
6. Update matrix/changelog/plan artifacts.
7. Repeat with the next dependency wedge.

## Prompt for the next implementation pass

> You are continuing the Omni project from the current repository state. Use
> `docs/archive/historical-plans/Omni_Complete_Specification_v3.4.md` and
> `docs/archive/historical-plans/Omni_v3.4_Third_Audit_Adjudication.md` as the
> normative spec/traceability pair.
>
> First, inspect the live implementation status and identify which behaviors are still
> qualified only in the current bootstrap line, which are explicitly fail-closed, and
> which are now superseded by the converged v3.4 rules.
>
> Then produce a dependency-aware implementation plan that batches compatible work
> items together instead of working on one isolated feature at a time. For each batch:
>
> * state the dependencies;
> * state the exact source, docs, tests, and conformance artifacts to touch;
> * state the expected failure modes;
> * state the qualification gates to run;
> * fix all failures before moving to the next batch.
>
> Preserve useful historical context, but do not let archived experiments define current
> semantics. Favor fail-closed behavior where the implementation is still behind the
> spec. Keep the changelog, versioning notes, and implementation matrix synchronized as
> you go.

