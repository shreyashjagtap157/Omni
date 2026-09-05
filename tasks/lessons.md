# Lessons Learned

## 2026-09-02 — Incomplete trait_system WIP broke compilation

**Pattern:** Threading a new subsystem (`TraitSystem`) into the type checker requires building it *before* the type-check step in `driver.rs`, and passing it through every `synthesize_expr` / `check_expr` / `check_stmts` call site — not just the top-level entry point.

**Rule:** Do not land partial refactors that reference undefined variables. Either complete the wiring in one change set or keep WIP on a feature branch. Run `cargo check` before ending a session.

## 2026-09-02 — Dead scaffolding fails clippy with `-D warnings`

**Pattern:** Adding `trait_bounds` / `record_trait_bound` fields without using them causes `dead_code` errors under workspace clippy settings.

**Rule:** Do not add placeholder fields for future milestones unless they are used in the same PR, or gate them behind a feature flag / `#[allow(dead_code)]` with an explicit milestone comment.

## 2026-09-02 — Qualification artifacts can go stale

**Pattern:** `BINARY_QUALIFICATION.json` claimed clippy/test PASS while HEAD had failures.

## 2026-09-05 — File operations tool preference

**Pattern:** Using terminal shell commands/scripts for common file operations when built-in agent tools (`view_file`, `write_to_file`, `replace_file_content`, etc.) are available.

**Rule:** Always prioritize built-in agent tools (`view_file`, `replace_file_content`, `write_to_file`, `list_dir`) for viewing, creating, updating, and moving files. Reserve terminal commands strictly for build, test, and git operations.
