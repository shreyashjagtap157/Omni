# Omni v3.4 Third Audit Adjudication

## Purpose

This document records the converged outcome of the third adversarial review cycle
discussed in the referenced conversation. It preserves the accepted corrections and
the reasons some stronger claims were rejected.

## Adjudication summary

### Accepted

1. Source order is observationally significant unless the compiler proves a safe
   transformation.
2. Reproducibility envelopes apply to artifacts and metadata, not to source semantics.
3. FFI lowering must preserve source evaluation order before ABI packing.
4. Semantic provenance and allocation identity must be preserved across lowering where
   required by the language model.
5. Freeze readiness is not implied by prose alone.

### Rejected

1. Any claim that ordering can be canonicalized simply because the final artifact is
   reproducible.
2. Any claim that argument evaluation can be reordered for deterministic output.
3. Any claim that the spec is frozen merely because a gate exists on paper.
4. Any invented qualification label or version path that is not already part of the
   repository's documented versioning scheme.

### Clarified

1. "Defined" is not "qualified".
2. "Formalized" is not "implemented".
3. "Implemented" is not "frozen".
4. Historical experiments remain useful references, but they do not override the
   converged specification.

## Residual issues intentionally carried forward

The third review did not attempt to erase all implementation gaps. Instead it clarified
what the current bootstrap must continue to enforce:

* fail closed on unsupported semantics;
* keep the current qualified surface narrow where necessary;
* avoid promising freeze or conformance without artifacts;
* keep the implementation plan dependency-aware and batched.

## Traceability notes

The review is intentionally aligned with the current repository status documents and the
bootstrap plan. If a later implementation batch changes one of the accepted items, that
batch must also update the traceability notes and the implementation matrix.

