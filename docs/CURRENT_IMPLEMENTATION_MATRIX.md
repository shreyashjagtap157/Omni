# Omni v0.1.4.1.1 implementation matrix

This is the current implementation-status authority for the Rust bootstrap. Syntax or
experimental helpers do not count as implementation unless the behavior is checked and
qualified through the canonical native path.

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

## Explicitly unqualified boundaries

| Area | Current rule |
|---|---|
| Aggregate field mutation | fail closed until ownership/place/drop semantics are qualified in v0.2.0 |
| Production ownership/borrowing/regions | v0.2.0 gate; archived Polonius adapters are not a soundness claim |
| Generic/trait semantics | five retained semantic tests intentionally deferred to v0.3.0 |
| Escaping stack slices | not allowed; v0.1.4.1.1 does not turn stack views into a general lifetime ABI |
| Generic source collections | not claimed; mutable aliasing requires ownership and generic semantics |
| Cranelift/LLVM execution | explicitly unqualified/fail closed |
| MLIR/Wasm | experiments only; not canonical language semantics |
| Stable FFI, async/concurrency, multi-ISA | later roadmap milestones |
| `build.omni` / remote dependency resolution | fail closed until hermetic/package-security milestone |

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
