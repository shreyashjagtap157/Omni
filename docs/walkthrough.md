# Omni Compiler Remediation — Session 2 Walkthrough

## Summary

This session completed **Phase 2** (Module Unification) and **Phase 5** (Pipeline Wiring), while discovering and fixing **4 critical bugs** in the lexer and parser that had been silently breaking the compilation pipeline.

## Phase 2: Module Unification

### Orphan Module Deletion

Analysis revealed that 4 source files existed in `crates/omni-compiler/src/` but were **never declared in `lib.rs`**, making them dead code that was never compiled:

| File | Status | Action |
|------|--------|--------|
| `bidirectional_typer.rs` | Not declared in `lib.rs` | **Deleted** |
| `effect_system.rs` | Not declared in `lib.rs` | **Deleted** |
| `trait_system.rs` | Not declared in `lib.rs` | **Deleted** |
| `levenshtein.rs` | Not declared in `lib.rs` | **Deleted** |
| `generational_refs.rs` | Declared in `lib.rs` | **Kept** (may be used externally) |

---

## Phase 5: Pipeline Wiring

### Resolver Integration

Wired `resolver::resolve_program()` into all main pipeline functions in [lib.rs](file:///d:/Project/Omni/crates/omni-compiler/src/lib.rs):

```diff
 pub fn run_file(path: &Path) -> Result<(), String> {
     let mut program = parse_file(path)?;
     inout_desugar::desugar_inout_in_ast(&mut program)?;
+    resolver::resolve_program(&program).map_err(|errs| errs.join("; "))?;
     type_checker::type_check_program(&program)?;
     interpreter::run_program(&program)
 }
```

Applied to: `run_file`, `check_file`, `emit_mir_file`, `check_mir_file`, `run_native_file`

---

## Critical Bug Fixes

### Bug 1: Lexer Raw String Prefix Greedy Consumption

> [!CAUTION]
> **Root Cause**: The `r#"..."#` raw string check consumed the `r` character **before** verifying `#` follows. This silently broke ALL identifiers/keywords starting with `r` (`return`, `ref`, `result`, etc.), splitting them into `r` + remaining text.

**Symptom**: `return 0` tokenized as `Ident("r")`, `Ident("eturn")`, `Number("0")`

**Fix in** [complete_lexer.rs](file:///d:/Project/Omni/crates/omni-compiler/src/complete_lexer.rs):
```diff
-if c == 'r' {
-    self.next_char();
+if c == 'r' && self.peek_nChar(1) == Some('#') {
+    self.next_char(); // Only consume 'r' when '#' follows
```

### Bug 2: Lexer Byte String Prefix Greedy Consumption

Same issue as Bug 1 but for `b"..."` byte strings — would break `break`, `bool`, etc.

```diff
-if c == 'b' {
-    self.next_char();
+if c == 'b' && self.peek_nChar(1) == Some('"') {
+    self.next_char();
```

### Bug 3: Lexer Stale Cursor After Indent Processing

> [!WARNING]
> **Root Cause**: After `indent_of()` consumed whitespace via `next_char()`, the loop variable `c` (captured at loop start by `peek_char()`) still held the **stale** space character. The subsequent `if c == ' '` check then consumed the first character of the actual token.

**Fix**: Added `continue` after indent/dedent processing to restart the loop with a fresh `peek_char()`.

### Bug 4: Parser Missing Keyword Token Dispatch

> [!IMPORTANT]
> **Root Cause**: The parser's `parse_statement()` only dispatched on `TokenKind::Ident` with text matching (e.g., `tok.text == "let"`). But the `complete_lexer` produces dedicated `TokenKind` variants (`TokenKind::Let`, `TokenKind::Fn`, `TokenKind::Return`, etc.). This meant the parser couldn't parse any keyword statements from complete_lexer tokens.

**Fix in** [parser.rs](file:///d:/Project/Omni/crates/omni-compiler/src/parser.rs): Added 15 keyword token handlers before the `TokenKind::Ident` fallback block.

### Additional Parser Fix: Curly Brace Block Support

The function body parser only accepted `[`/`]` (square brackets) and `Indent`/`Dedent` blocks. Added `LBrace`/`RBrace` support, fixing 50 generated regression tests that use `fn foo() { ... }` syntax.

---

## Test Results

| Suite | Result |
|-------|--------|
| Generated regressions | **200/200** ✅ (was 150/200) |
| Borrow check UI | **6/6** ✅ (was 0/6) |
| Type inference UI | ✅ |
| MIR optimize | ✅ |
| Codegen LIR | ✅ |
| Debug tokens | ✅ |
| Public API effects | ✅ |
| Layout edge cases | **4/5** (1 pre-existing failure: `block_comments_preserved`) |
| **Total workspace** | **1 pre-existing failure** |

## Files Modified

| File | Changes |
|------|---------|
| [complete_lexer.rs](file:///d:/Project/Omni/crates/omni-compiler/src/complete_lexer.rs) | Fixed 3 critical bugs (raw string prefix, byte string prefix, stale cursor) |
| [parser.rs](file:///d:/Project/Omni/crates/omni-compiler/src/parser.rs) | Added keyword token dispatch, curly brace block support |
| [lib.rs](file:///d:/Project/Omni/crates/omni-compiler/src/lib.rs) | Wired resolver into 5 pipeline functions |
| 4 test files | Fixed double-unwrap (`tokenize().expect().unwrap()` → `tokenize().expect()`) |
| 3 test files | Fixed corrupted import `complete_complete_lexer` |
| 2 test files | Fixed garbled tokenization calls |

## Files Deleted
- `bidirectional_typer.rs`, `effect_system.rs`, `trait_system.rs`, `levenshtein.rs`
