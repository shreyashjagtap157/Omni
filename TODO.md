# TODO: Fix Steps 1.1-1.3 Gaps, Then Implement Step 1.4/1.5

Based on user approval: "First point 1, then 2" — fix gaps in steps 1.1-1.3, then proceed to step 4.

## Phase 1: Fix Gaps in Steps 1.1–1.3

### Step 1.1 Gaps — Expand TokenKind to full spec set
- [x] Add missing spec keywords to `TokenKind` enum:
  - `async`, `await`, `comptime`, `effect`, `trait`, `impl`, `extern`, `inout`, `yield`, `spawn`, `use`, `mod`, `cap`, `friend`
- [x] Add keyword matching in lexer's `tokenize()` method for all new keywords
- [x] Fix compound assignment operator tokenization (`+=`, `-=`, `*=`, `/=`, `%=`):
  - Currently declared in `TokenKind` but never emitted by lexer
  - Update `'+'`, `'-'`, `'*'`, `'/'` branches to check for trailing `'='`

### Step 1.2 Gaps — Implement numeric literals fully
- [x] Fix `0.x` float parsing bug (numbers starting with `0` followed by decimal point)
- [x] Add integer numeric suffix support (`u8`, `i32`, `u64`, `usize`, `isize`, etc.)
- [x] Add underscore separator support (`1_000_000`)
- [x] Add exponent notation support (`1e10`, `1.5e-3`, etc.)

### Step 1.3 Gaps — Implement raw strings and byte strings
- [x] Raw string tokenization is already implemented in the `r` branch.
- [x] Add byte string tokenization: `b"..."` form
- [x] Add byte string variant to `TokenKind` (or re-use `StringLiteral` with byte content marker)

## Phase 2: Implement Step 1.4/1.5 from Implementation Plan

### Step 1.4 — Implement attribute tokens and braces
- [x] Verify `@`, `{`, `}`, `;` are fully functional in lexer (already present — confirm no gaps)

### Step 1.5 — Build complete_lexer.rs with layout engine (Next Step)
- [x] Evaluate if current `lexer.rs` layout engine (Indent/Dedent) is sufficient
- [x] Improve layout/indentation handling if needed
- [x] Add comprehensive lexer tests for all new features

## Acceptance Criteria
- [ ] `cargo test --workspace` passes after all changes
  - Currently blocked by pre-existing resolver compile errors in `crates/omni-compiler/src/resolver.rs` (`Rc<Scope>` mutable borrow errors).
- [x] All new TokenKind variants are tokenizable and testable
- [x] Numeric literals cover all specified forms correctly
- [x] Byte strings parse correctly
