---
name: typescript-lsp
description: TypeScript and JavaScript language-server workflow for diagnostics, symbol-aware refactors, and project-aware verification. Use when working in .ts, .tsx, .js, .jsx, .mts, .cts, .mjs, or .cjs files and you need language-aware validation or safer code navigation.
---

# TypeScript LSP

Use this skill when working in TypeScript or JavaScript code that benefits from language-aware tooling.

## When To Use

- Use it for changes in `.ts`, `.tsx`, `.js`, `.jsx`, `.mts`, `.cts`, `.mjs`, or `.cjs` files.
- Use it when a task involves type errors, imports, symbol renames, definitions, references, or project configuration.
- Prefer it over plain text editing when a change touches exported APIs, shared types, or widely reused symbols.

## Workflow

1. Inspect diagnostics for the affected files before editing so the existing failure mode is clear.
2. Use language-aware navigation to find definitions, references, implementations, and related types before changing shared code.
3. Prefer symbol-aware rename and refactor operations over text-only search and replace.
4. Make the smallest change that fixes the issue at the correct abstraction level.
5. Re-run project-aware verification after meaningful edits and resolve any new diagnostics before concluding work.

## Core Practices

- Prefer rename, references, go-to-definition, and implementation lookups over manual scanning.
- Preserve existing tsconfig or jsconfig behavior, path aliases, module resolution, and import style.
- Avoid large mechanical rewrites unless the task explicitly requires them.
- Be careful with exported types, discriminated unions, overloads, generic constraints, and shared utility types.
- Distinguish editor diagnostics from full project verification: a file can look valid in isolation but still fail under the workspace configuration.
- Avoid ad hoc single-file compilation when the project relies on workspace config, path mapping, JSX settings, or bundler-specific module resolution.

## Verification

Prefer the project's existing verification entry points. In most codebases that means one or more of:

- a package script such as `npm run typecheck`, `pnpm typecheck`, or `yarn typecheck`
- `tsc --noEmit -p <tsconfig>` for project-aware type checking
- the affected test suite when the change touches runtime behavior as well as types
- the project's lint command when import resolution or unused-symbol rules matter

Prefer project-aware verification over commands like `tsc some-file.ts`, which often bypass the real workspace configuration.

## Notes

This skill is intentionally generic and portable. Adapt the final verification command to the current workspace instead of assuming a fixed package manager, script name, or tsconfig path.