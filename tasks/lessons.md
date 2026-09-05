# Lessons Learned

## 2026-09-02 — Incomplete trait_system WIP broke compilation

**Pattern:** Threading a new subsystem (`TraitSystem`) into the type checker requires building it *before* the type-check step in `driver.rs`, and passing it through every `synthesize_expr` / `check_expr` / `check_stmts` call site — not just the top-level entry point.

**Rule:** Do not land partial refactors that reference undefined variables. Either complete the wiring in one change set or keep WIP on a feature branch. Run `cargo check` before ending a session.

## 2026-09-02 — Dead scaffolding fails clippy with `-D warnings`

**Pattern:** Adding `trait_bounds` / `record_trait_bound` fields without using them causes `dead_code` errors under workspace clippy settings.

**Rule:** Do not add placeholder fields for future milestones unless they are used in the same PR, or gate them behind a feature flag / `#[allow(dead_code)]` with an explicit milestone comment.

## 2026-09-02 — Qualification artifacts can go stale

**Pattern:** `BINARY_QUALIFICATION.json` claimed clippy/test PASS while HEAD had failures.

## 2026-09-05 — Strict Prohibition of Mutating File Operations via Terminal Scripts

**Pattern:** Attempting or performing file creation, editing, overwriting, moving, or deletion using terminal shell scripts, PowerShell commands, Python one-liners, or code execution.

**Rule:** NEVER perform any file operations (create, write, edit, overwrite, rename, move, delete) using terminal scripts, shell commands, or code execution. ONLY read operations (e.g., checking git status or running build/test/audit runners) are allowed via terminal commands. ALL mutating file operations MUST be executed exclusively using the agent's built-in tools (`write_to_file`, `replace_file_content`, `multi_replace_file_content`).
