# Repository Guidelines

## Project Structure & Module Organization

This repository is a small Rust CLI project.

- `src/main.rs`: main application entry point, CLI parsing, registry resolution, Git clone flow, and `.luarc.json` editing.
- `README.md` and `README_ZH.md`: English and Chinese user documentation. Keep both aligned when behavior changes.
- `playground/`: local manual test area with example `.luarc.json` and addon output.
- `target/`: generated build artifacts. Do not edit or review generated files there.

Unit tests currently live in `src/main.rs` under `#[cfg(test)]`. If the code grows, move logic into modules under `src/` and keep tests close to the code they exercise.

## Build, Test, and Development Commands

- `cargo build --release`: build the optimized CLI binary.
- `cargo test`: run unit tests, including JSONC and `.gitmodules` parsing coverage.
- `cargo fmt`: format Rust code with the standard formatter.
- `cargo run -- --help`: inspect CLI usage during development.
- `cargo run -- --target ./LuaAddons openresty`: local end-to-end check against the official addon registry.

## Coding Style & Naming Conventions

Use standard Rust style with 4-space indentation and `cargo fmt` formatting. Prefer descriptive snake_case for functions and variables, and PascalCase for types. Keep CLI-facing error messages specific and actionable. When changing `.luarc.json` behavior, preserve JSONC comments and formatting whenever possible.

## Testing Guidelines

Add or update unit tests for every change to:

- addon registry parsing
- `.luarc.json` update behavior
- JSONC preservation logic

Name tests by behavior, for example `accepts_jsonc_luarc_with_comments_and_trailing_commas`. Run `cargo test` before opening a PR.

## Commit & Pull Request Guidelines

Follow Conventional Commits. Current history uses messages such as:

- `feat: add LuaLS addon installer with JSONC-aware .luarc updates and bilingual docs`

PRs should include a short summary, user-visible behavior changes, test results, and any documentation updates. If CLI output, config rewriting, or README examples change, include the updated command example in the PR description.

## Configuration Notes

Do not hardcode personal paths in docs or examples. Prefer project-local paths like `./LuaAddons`. Treat `.luarc.json` edits carefully because this tool modifies contributor configuration files.
