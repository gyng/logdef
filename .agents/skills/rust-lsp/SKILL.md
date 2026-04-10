---
name: rust-lsp
description: Rust language-server workflow for diagnostics, symbol-aware refactors, and workspace-aware verification. Use when working in .rs files and you need rust-analyzer style navigation, safer renames, or Rust-specific validation.
---

# Rust LSP

Use this skill when working in Rust code that benefits from language-aware tooling.

## When To Use

- Use it for changes in `.rs` files, especially when types, traits, modules, or public APIs are involved.
- Use it when a task involves borrow-checker issues, trait resolution, enum or struct changes, or cross-module refactors.
- Prefer it over plain text editing when a symbol is reused across crates or modules.

## Workflow

1. Inspect diagnostics for the affected files and modules before editing so the existing failure mode is clear.
2. Use language-aware navigation to find definitions, references, implementations, trait impls, and call sites before changing shared code.
3. Prefer symbol-aware rename and refactor operations over text-only search and replace.
4. Make the smallest change that fixes the issue at the right ownership, trait, or module boundary.
5. Re-run workspace-aware verification after meaningful edits and resolve any new diagnostics before concluding work.

## Core Practices

- Prefer symbol rename and reference tools over search-and-replace for structs, enums, traits, functions, and shared fields.
- Preserve existing crate boundaries, module layout, visibility, feature flags, and naming conventions.
- Avoid broad mechanical rewrites unless the task explicitly requires them.
- Be careful with lifetimes, ownership moves, trait bounds, associated types, and pattern matches that may require coordinated updates.
- When changing public types or traits, inspect downstream uses before finalizing the edit.
- Treat formatting, clippy, and tests as complementary checks rather than substitutes for `cargo check`.

## Verification

Prefer the project's existing cargo entry points. In most Rust codebases that means one or more of:

- `cargo check` or `cargo check --workspace` for compile validation
- `cargo test` or targeted tests when behavior or regressions may have changed
- `cargo clippy` when the project uses clippy as part of normal validation
- `cargo fmt --check` or the project's formatting command when formatting consistency matters
- target-specific builds when the change affects a particular binary, crate, or compilation target

Prefer cargo-based verification over direct `rustc` invocation or ad hoc per-file compilation.

## Notes

This skill is intentionally generic and portable. Adapt the final verification command to the current workspace instead of assuming a fixed cargo target, workspace layout, or lint policy.