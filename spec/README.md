# Omni specification baseline used by this implementation

This source tree carries a snapshot of the Edition 1 language-definition candidate
under `spec/edition1/` so compiler work is tied to a concrete semantic target.

The snapshot is **not a ratified final standard**. It is the current architecture and
language-definition target from the design work preceding this implementation. A
subsequent adversarial review found that some advanced areas still require stronger
formalization and executable conformance evidence. Implementation work therefore uses
these rules conservatively:

1. Native AOT is the canonical execution model.
2. Safe observable behavior is defined; the compiler must fail closed rather than
   invent semantics for an unimplemented feature.
3. Rust is a bootstrap implementation dependency only. Generated native programs do
   not require Rust, Cargo, a VM, JIT, or LLVM runtime.
4. Experimental JIT/LLVM/MLIR/Wasm backends are secondary validation/development
   paths and never define the language.
5. The implementation status document records exactly which Edition 1 features are
   end-to-end versus parser/type-checker scaffolding.

A future ratified standard should replace the candidate snapshot without making the
production compiler implementation itself the semantic authority.
