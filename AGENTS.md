# Repository Guidelines

## Project Structure & Module Organization

`lazyssh` is a single-package Rust 2021 terminal application. `src/main.rs` initializes the terminal, runs the event loop, and restores terminal state. Core responsibilities are split across `src/app.rs` (application state), `src/event_handler.rs` (keyboard and mouse input), `src/render.rs` (Ratatui UI), `src/server.rs` (SSH config parsing and server models), and `src/ssh_login.rs` (login execution). Project metadata and dependencies live in `Cargo.toml`; `Cargo.lock` is committed for reproducible application builds. `README.md` documents user-facing behavior, while `demo.gif` is the visual demo. CI packaging is defined in `.github/workflows/rust.yml`.

## Build, Test, and Development Commands

- `cargo run` builds and launches the TUI against the current user's `~/.ssh/config`.
- `cargo build` performs a debug build for routine development.
- `cargo build --release` matches the build performed by the packaging workflow.
- `cargo test` runs the unit tests. Some server parsing tests read `~/.ssh/config`, so use a controlled fixture or environment when extending them.
- `cargo fmt --check` verifies standard Rust formatting without rewriting files.
- `cargo clippy --all-targets --all-features -- -D warnings` reports lint issues and treats warnings as failures.

## Coding Style & Naming Conventions

Use standard `rustfmt` output and four-space indentation. Follow Rust naming conventions: `snake_case` for modules, functions, and variables; `PascalCase` for structs and enums; and `SCREAMING_SNAKE_CASE` for constants. Keep modules focused on their existing responsibilities and prefer small, direct changes over new abstractions.

## Testing Guidelines

Place focused unit tests beside the code under `#[cfg(test)]`. Name tests after observable behavior, for example `parses_grouped_hosts`, rather than generic names such as `test`. Add regression coverage for parser, filtering, selection, and event-handling changes. There is no documented coverage threshold; prioritize deterministic assertions and avoid depending on real credentials or network access.

## Commit & Pull Request Guidelines

Recent history primarily uses Conventional Commit prefixes such as `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, and `style:`. Use a concise imperative summary and avoid vague messages. Pull requests should explain the behavior change, list verification commands, and link relevant issues. Include a screenshot or short recording for visible TUI changes. Never commit SSH private keys, passwords, or a personal `~/.ssh/config`.
