# Omni v3.4 reconciliation matrix

This document is the current implementation-status authority for the Rust bootstrap
reconciled against the converged v3.4 specification archive. Syntax or experimental
helpers do not count as implementation unless the behavior is checked and qualified
through the canonical native path.

The live bootstrap line remains v0.1.4.1.1 for currently qualified native behavior,
but the repository now tracks that status against the stricter v3.4 rules.

## v3.4 gap summary

The main gaps between the current bootstrap and the converged v3.4 spec are:

* source-order observability and optimizer proof obligations are now backed by a MIR lowering regression for nested call arguments plus conservative optimizer barriers for calls, spawns, indirect writes, and subobject writes, but native corpus coverage still needs expansion;
* reproducibility-envelope rules exist conceptually, but the repository does not yet
  expose a dedicated conformance corpus that proves they cannot alter source meaning;
* FFI/ABI lowering order is partially covered by native value-ABI work, but the v3.4
  traceability still needs explicit negative coverage for semantic reordering;
* provenance and allocation-identity language exists in the spec, but the current matrix
  does not yet record a complete per-pass provenance-preservation audit;
* freeze/qualification state is still documented with milestone language, not a full
  v3.4 `Defined -> Formalized -> Implemented -> Qualified -> Frozen` traceability model;
* the repository still relies on separate historical and milestone docs instead of a
  single current gap list tied directly to v3.4.

## Qualified cumulative native core

| Area | Current status | Canonical/native status | Next gate |
|---|---|---|---|
| Source/lexer/parser | Broad bootstrap grammar; canonical scalar/layout/value-ABI forms covered by corpora | Qualified only for cumulative exercised subset | Edition-gated normative grammar + Unicode/security corpus |
| Scalar types | integer/bool plus `byte` scalar path | Qualified subset | complete numeric/coercion semantics |
| Control flow | calls, `if`, loops, break/continue, checked arithmetic | Qualified from v0.1.2 | richer expression/control semantics |
| MIR | typed params/returns, scalar/local aggregate/enum/String/Bytes operations | Qualified cumulative subset | place/ownership/effect invariants |
| LIR | typed values, `Ptr(cells)` bounded indirect ABI, checked local/index operations, String/Bytes refs | **Qualified v0.1.4.1.1 subset** | target-general ABI/value classes |
| Native AOT | owned x86-64 Linux ELF64 encoder/writer | **Canonical v0.1.4.1.1 path** | other object formats/ISAs |
| Structs | nominal scalar-cell local layout plus bounded indirect argument/return transport | **Qualified subset** | nested/nontrivial fields + ownership |
| Tuples/arrays | local scalar-cell layout; checked indexing | Qualified local subset | general aggregate ABI/richer elements |
| Local slices | checked non-escaping local views | Qualified local subset | lifetime-aware escaping/general slice ABI |
| Enums | nominal tag/payload local layout; exhaustive match; scalar payload ABI crossing | **Qualified subset** | richer payloads/patterns |
| String | immutable `{data,len}` UTF-8 descriptor; pass/return, `.len`, runtime print | **Qualified v0.1.4.1.1 subset** | owned/mutable strings, Unicode character/grapheme APIs |
| `byte` | 0-255 primitive literal/value path; scalar call ABI | **Qualified v0.1.4.1.1 subset** | complete numeric conversion rules |
| Bytes | binary `{data,len}` descriptor; arbitrary binary literal/pass/return/print; checked byte indexing | **Qualified v0.1.4.1.1 subset** | owned/mutable byte buffers |
| UTF-8 String indexing | direct `String[index]` rejected | **Fail closed** | explicit byte/character view semantics |
| Aggregate ABI | bounded pointer params; caller-owned hidden return storage; validated cell spans | **Qualified scalar-cell subset** | stable general ABI + nested/nontrivial values |
| Allocator foundation | `CellAllocator`, bootstrap host allocator, allocation grow/shrink/deallocate contract | **Qualified Rust-bootstrap foundation** | language-visible allocator/capability contract |
| Collection foundation | `OmniCellVector` checked scalar cells, reserve/shrink, failure-atomic growth | **Bootstrap foundation only** | ownership-safe source collections/generics |

## v3.4 semantic traceability

| v3.4 commitment | Current implementation status | Notes |
|---|---|---|
| Source order is preserved unless observational equivalence is proven | Partially implemented and regression-tested | MIR lowering preserves nested call-argument order; constant propagation invalidates facts at calls, spawns, indirect writes, and provenance-sensitive subobject writes; native corpus coverage remains incomplete |
| Reproducibility cannot change meaning | Documented, not comprehensively proven | Current release tooling should remain metadata-only |
| FFI/ABI evaluation order is source-order-sensitive | Partially covered by native ABI lowering | Needs negative tests for reordering and canonicalization attempts |
| Provenance and allocation identity are semantics, not just representation | Documented in code/spec | Needs per-pass audit coverage in the matrix |
| Unsupported behavior fails closed | Mostly enforced | Still needs spec-to-test mapping for newer v3.4 rules |
| Freeze requires artifacts, not prose | Not yet modeled as a separate gate | Current milestone docs should not imply freeze without evidence |

## Explicitly unqualified boundaries

| Area | Current rule |
|---|---|
| Aggregate field mutation | **Qualified in v0.2.0.0**: direct field assignment (`p.x = expr`) for `let mut` and linear field reinitialization |
| Production ownership/borrowing/regions | **Qualified in v0.2.0.0**: Polonius borrow checking, non-lexical lifetimes, multi-block reference tracking, linear place CFG analysis, fail-closed escape checks |
| Generic/trait semantics | five retained semantic tests intentionally deferred to v0.3.0 |
| Escaping stack slices | not allowed; v0.2.0.0 does not turn stack views into a general lifetime ABI |
| Generic source collections | not claimed; mutable aliasing requires ownership and generic semantics |
| Cranelift/LLVM execution | explicitly unqualified/fail closed |
| MLIR/Wasm | experiments only; not canonical language semantics |
| Stable FFI, async/concurrency, multi-ISA | later roadmap milestones |
| `build.omni` / remote dependency resolution | fail closed until hermetic/package-security milestone |

## v3.4 implementation gaps by dependency wedge

1. Source semantics and optimizer proofs, including native conformance for the lowering-order regression and optimizer memory barriers.
2. Provenance-preserving lowering through MIR/LIR/native codegen.
3. ABI/FFI evaluation order and negative semantic-reordering tests.
4. Qualification-corpus expansion for the new spec rules.
5. Documentation, changelog, and status synchronization for v3.4 traceability.

## Qualification contract

A v0.1.4.1.1 promotion must pass the pinned Rust 1.97.1 workspace source gates plus:

- historical v0.0.1 compatibility: 5 cases;
- native scalar v0.1.2: 23 cases;
- native layout v0.1.3: 10 cases;
- native value ABI v0.1.4.1.1: 10 cases;
- at least 60 cumulative seconds of deterministic lexer/parser fuzzing;
- zero ignored tests in the claimed v0.1.4.1.1 surface (the retained ignores are v0.3.0 trait semantics only).

The implementation advances by semantic wedges: source checking -> MIR -> verification ->
LIR -> native execution -> external conformance. A frontend-only feature is not counted as
complete.

For the v3.4 reconciliation, the qualification framing is:

* Defined: the rule exists in `docs/archive/historical-plans/Omni_Complete_Specification_v3.4.md`.
* Formalized: the repository carries an explicit status or traceability note for the rule.
* Implemented: code paths enforce the rule.
* Qualified: a corpus or gate proves the rule on the current bootstrap path.
* Frozen: the rule has explicit release evidence and is locked for the line.
