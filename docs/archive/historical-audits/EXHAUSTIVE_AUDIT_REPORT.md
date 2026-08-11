# EXHAUSTIVE AUDIT REPORT: Omni Compiler Pipeline

## Executive Summary

This report synthesizes the current audit results for the Omni compiler pipeline. While the structural skeleton of the pipeline is integrated, there are critical implementation gaps in memory lowering and semantic verification that prevent the compiler from being production-ready.

### Pipeline Status Overview

| Stage | Name | Status | Test Coverage | Verdict |
| :--- | :--- | :--- | :--- | :--- |
| 1 | Lexing | ✅ Implemented | Partial (Smoke) | Structurally sound; requires rigorous testing. |
| 2 | Parsing | ⚪ Not Audited | - | - |
| 3 | Resolving | ⚪ Not Audited | - | - |
| 4 | Type Checking | ⚪ Not Audited | - | - |
| 5 | Effect Resolution | 🟡 Partial | Low/Medium | Tracking works; algebraic handlers missing. |
| 6 | Traits | ⚪ Not Audited | - | - |
| 7 | MIR Generation | ⚪ Not Audited | - | - |
| 8 | MIR Optimization | ✅ Implemented | Partial | Functional; pipeline order conflict detected. |
| 9 | Borrow Checking | ⚪ Not Audited | - | - |
| 10 | LIR Lowering | 🟡 Partial | Low | **CRITICAL**: Silent failures on memory access. |
| 11 | Code Generation | ⚪ Not Audited | - | - |

---

## Detailed Stage Analysis

### Stage 1: Lexing
**Status:** Implemented
**Verdict:** High compliance with v2.0 specification.

#### Findings
The Lexing stage is highly precise and correctly handles Omni's complex syntax, including the indentation stack and multiple literal types. Integration into `driver.rs` is correct, and error propagation is functioning as expected.

#### Gaps & Risks
- **Testing Deficit**: Coverage is significantly below the mandatory 85% requirement. Current tests are "smoke tests" rather than comprehensive unit tests.
- **Missing Invariants**: No property-based roundtrip tests (`Lex -> Format -> Lex`) are implemented.
- **Edge Case Exposure**: Lack of exhaustive testing for nested indentation and complex raw string boundaries.

---

### Stage 5: Effect Resolution
**Status:** Partial
**Verdict:** Functional effect tracking, but fails to implement the "Algebraic" nature of the spec.

#### Findings
The `EffectResolver` successfully implements fixed-point iteration over the call graph to propagate effects (`io`, `async`, `panic`, `pure`). Integration into the pipeline (Stage 5) is correct. Runtime support for structured concurrency (Channels, SpawnScope) is present.

#### Gaps & Risks
- **Spec Non-Compliance**: The implementation is a traditional effect-tracking system. It lacks **algebraic effect handlers** (resumption and effect capture) as required by the v2.0 spec.
- **Logic Error**: `verify_entry_point` is too restrictive; it currently flags non-pure effects in `main` as errors, which contradicts the purpose of a program entry point.
- **Verification Gap**: Integration tests for the `EffectResolver` stage in `driver.rs` are missing; tests currently bypass the resolver by calling the type checker directly.

---

### Stage 8: MIR Optimization
**Status:** Implemented
**Verdict:** Functional logic, but introduces architectural risks regarding pipeline order.

#### Findings
Core optimizations (Constant Folding, DCE, and Simple Function Inlining) are fully implemented and integrated. No `unimplemented!()` macros exist in the execution path.

#### Gaps & Risks
- **Pipeline Order Conflict**: 
    - **Spec**: `Borrow Checker` $\rightarrow$ `MIR Optimization`.
    - **Driver**: `MIR Optimization` $\rightarrow$ `Borrow Checker`.
    - *Risk*: Inlining changes the CFG; performing it before the borrow checker may invalidate assumptions or hide bugs that the borrow checker should have caught.
- **Technical Debt**: Contains dead code stubs in `mir_optimize.rs` (`try_constant_fold`, `eval_var`) that should be removed per `production_code_standard.md`.
- **Testing Depth**: Test suite lacks coverage for complex CFG interactions and optimization regressions.

---

### Stage 10: LIR Lowering
**Status:** Partial
**Verdict:** **CRITICAL FAILURE**. Produces incorrect binaries for non-scalar data.

#### Findings
The LIR lowering stage is integrated but fails to implement the lowering of essential memory and ownership instructions.

#### Gaps & Risks
- **Silent Instruction Loss**: The following instructions are tracked for slots but are silently mapped to `LirInstr::Nop` during lowering:
    - `LinearMove`
    - `FieldAccess`
    - `StructAccess`
    - `IndexAccess`
- **Production Standard Violation**: The use of a catch-all `_ => LirInstr::Nop` hides missing implementations, leading to silent failures instead of explicit compiler errors.
- **Verification Gap**: Tests only cover basic arithmetic. Programs using structs or arrays are not validated for correctness.

---

## Critical Path to v1.0.0

The following items are prioritized by their impact on correctness and specification compliance.

### Priority 1: Correctness (Blockers)
1. **Fix LIR Memory Lowering**: Implement `LinearMove`, `FieldAccess`, `StructAccess`, and `IndexAccess`.
2. **Eliminate Silent Failures**: Replace `LirInstr::Nop` fallbacks in `codegen_lir.rs` with `unimplemented!()` or proper diagnostics to prevent silent code corruption.
3. **Resolve Pipeline Order**: Align `driver.rs` with the specification: Move `Borrow Checking` before `MIR Optimization` to ensure CFG transformations are validated.

### Priority 2: Specification Compliance
1. **Implement Algebraic Effects**: Develop effect handlers and resumption logic in the `EffectResolver` to move beyond simple tracking.
2. **Fix Entry Point Logic**: Update `verify_entry_point` to allow IO and other effects within the `main` function.

### Priority 3: Quality & Verification
1. **Lexer Test Suite**: Implement property-based roundtrip tests and expand coverage to 85% for indentation and raw strings.
2. **Effect Integration Testing**: Create dedicated tests for the `EffectResolver` stage that run through the full `driver.rs` pipeline.
3. **MIR Opt Clean-up**: Remove dead code stubs from `mir_optimize.rs`.
