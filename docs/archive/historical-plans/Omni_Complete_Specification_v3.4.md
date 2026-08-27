# Omni Complete Specification v3.4

## Status

This document captures the converged specification outcome of the latest adversarial
review cycle discussed in the referenced ChatGPT conversation. It is the normative
archive for the v3.4 rule set and supersedes earlier draft language wherever it is
more specific.

This is a specification document, not an implementation claim. A rule may be defined
here before it is implemented in the current Rust bootstrap. When implementation
coverage is missing, the compiler and runtime must continue to fail closed.

## 1. Normative hierarchy

The project uses five progression states:

1. `Defined` - the rule exists in the specification.
2. `Formalized` - the rule has an explicit machine-checkable shape.
3. `Implemented` - the code path exists.
4. `Qualified` - the code path has been exercised by the project qualification gates.
5. `Frozen` - the behavior is locked for the current release line.

Specification prose can establish `Defined` and, where the document is precise enough,
`Formalized`. It cannot by itself establish `Implemented`, `Qualified`, or `Frozen`.

## 2. Specification authority

The repository's current living target may lag behind this v3.4 archive.
Implementation work must therefore distinguish:

* spec-defined behavior;
* currently qualified bootstrap behavior;
* explicitly fail-closed unsupported behavior;
* archived historical experiments that are not normative.

Where the codebase and this document differ, the current implementation must not invent
semantics to bridge the gap.

## 3. Converged rules

### 3.1 Source order and observability

Source semantics are evaluated in source order unless the compiler proves an
observational-equivalence transformation. Optimizers do not get to reorder or drop
operations merely because the compiler author believes the result is "pure enough".

In particular:

* observable reads and writes preserve program order unless a proof authorizes change;
* I/O is not assumed pure;
* "deterministic" does not mean "semantically reorderable";
* reordering requires explicit evidence of equivalence under the language model.

### 3.2 Reproducibility does not alter meaning

Reproducibility requirements apply to the emitted artifact, not to source semantics.
The compiler may canonicalize metadata, build envelopes, hashes, and packaging details.
It may not canonicalize argument evaluation, control flow, or value selection in a way
that changes program meaning.

### 3.3 FFI and ABI evaluation order

FFI lowering preserves the language evaluation order and then packs values according to
the declared ABI.

Required rule:

* evaluate argument 1;
* evaluate argument 2;
* evaluate the remaining arguments in source order;
* then place the resulting values into ABI-defined locations.

Forbidden rule:

* sorting arguments or otherwise reordering evaluation for reproducibility;
* changing source order to match a canonical encoding;
* treating ABI packing as permission to rewrite semantic order.

### 3.4 Provenance and identity

Semantic provenance is the identity required to preserve meaning across lowering,
optimization, storage, and reconstruction. Representation choice is implementation-
defined only when it preserves that meaning.

The implementation may choose any internal encoding that preserves:

* allocation identity where required;
* subobject identity where required;
* recovery rules for raw exposure;
* provenance-preserving lowering through IRs;
* failure-closed behavior where recovery is not sound.

### 3.5 Alignment, layout, and canonical lowerings

The compiler may canonicalize a representation only when canonicalization is semantics-
preserving. Layout canonicalization is permitted for envelopes, snapshots, and release
artifacts. It is not a license to invent a new source meaning.

### 3.6 Freeze gating

Freeze readiness is achieved only when the corresponding artifacts actually exist and
have been verified. A textual specification alone does not satisfy qualification.

Frozen means:

* the behavior has concrete test artifacts;
* the behavior has been exercised by the appropriate gate;
* the project has chosen to stop changing it in the current release line.

### 3.7 Fail-closed policy

Unsupported behavior must fail explicitly rather than silently degrade into a guessed
implementation. This applies to parser support, lowering, optimization, runtime
execution, FFI, and tooling.

## 4. Current converged architectural commitments

The following commitments remain in force:

* source semantics remain source-order-sensitive unless proven otherwise;
* optimizer transformations require observational-equivalence justification;
* provenance-preserving lowering is mandatory for any raw-pointer or allocation-
  identity-sensitive representation;
* the implementation must keep the current bootstrap target fail-closed for unqualified
  features;
* semantic freeze is distinct from documentation freeze;
* reproducibility envelopes cannot mutate program behavior;
* the current implementation may be narrower than the spec and still be correct if it
  refuses unsupported constructs.

## 5. Residual implementation expectations

The current repository still needs to reconcile its v0.x bootstrap line with the v3.4
convergence model. The main expectation is not "implement everything at once" but
"expand coverage in compatible batches without violating fail-closed boundaries".

That means future work should prefer:

* adjacent source/IR/runtime steps that share dependencies;
* conformance evidence that covers a batch, not a single isolated syntax point;
* plan documents that explicitly record what is implemented, qualified, and deferred;
* compatibility notes that distinguish superseded experiments from current rules.

## 6. Relationship to other docs

This archive should be read together with:

* `Omni_v3.4_Third_Audit_Adjudication.md`
* `Omni_v3.4_Implementation_Plan.md`
* `../../CURRENT_IMPLEMENTATION_MATRIX.md`
* `../../VERSIONING_AND_BOOTSTRAP_PLAN.md`
* `../../spec/README.md`

## 7. Practical summary

The v3.4 conclusion is simple:

* the spec is stricter than the current bootstrap in several places;
* the bootstrap must continue failing closed where it has not caught up;
* reproducibility, provenance, and ABI rules cannot rewrite source semantics;
* freeze claims require artifacts, not just prose;
* the next implementation step should be a batched dependency-aware reconciliation.

