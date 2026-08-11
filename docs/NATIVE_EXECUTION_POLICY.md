# Omni native execution policy

## Normative implementation direction

Omni's canonical deployment model is ahead-of-time native compilation. A conforming
production implementation must not require an Omni VM, JVM-like runtime, Python-like
interpreter loop, or mandatory JIT between a released program and its target machine.

Hosted path:

```
source -> semantic IR -> native ISA/object/executable -> OS loader -> processor
```

Freestanding path:

```
source -> semantic IR -> native ISA/freestanding image -> firmware/boot -> processor/devices
```

The operating system, firmware, drivers, and hardware itself are real platform layers;
Omni does not pretend they disappear.

## Bootstrap implementation versus generated program

Through the Rust-bootstrap era, `omnic`/`omni` may itself be a Rust-built executable.
That does **not** permit generated Omni applications to depend on the Rust runtime or
Cargo. Rust is an implementation dependency of the compiler until self-hosting.

The owned native backend should progressively take responsibility for:

- instruction encoding;
- object/executable/image writing;
- relocations and symbols;
- static/dynamic linking policy;
- startup code;
- platform ABI lowering;
- debug/unwind metadata;
- target-feature validation.

External assemblers/linkers may be temporary development or comparison tools, but are
not the end-state Tier-1 bootstrap dependency.

## Development oracles

The following may exist without changing canonical semantics:

- MIR reference interpreter;
- Cranelift JIT for REPL/debugging/differential testing;
- LLVM comparison backend;
- MLIR research lowering;
- Wasm portable/sandbox target.

No such backend defines the language. Differential disagreement is a compiler defect
or specification question to resolve, not permission for backend-specific semantics.

## Direct hardware

The `bare_metal` profile will ultimately expose typed, capability-gated facilities for:

- MMIO and device-register mappings;
- interrupts and exception vectors;
- DMA memory and IOMMU contracts;
- physical/virtual address spaces;
- page tables and TLB/cache operations where permitted;
- ISA-specific control/status registers and intrinsics;
- atomics, fences, and device-ordering operations;
- linker sections, reset/entry vectors, firmware/boot interfaces;
- architecture-specific privileged instructions inside explicit unsafe/authority
  boundaries.

Direct hardware access does not mean untyped global pointer authority. Unsafe/device
operations remain narrow because a systems language should make kernels possible, not
make accidental page-table demolition ergonomic.
