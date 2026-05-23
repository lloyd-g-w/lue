# Repository Guidelines

## Project Structure & Module Organization

This is a Rust workspace for a Dioxus + Axum live queue app. Crates live under `crates/`:

- `crates/server`: Axum backend, WebSocket handling, persistence, password hashing, and queue store logic.
- `crates/web`: Dioxus web frontend, routes, pages, components, client storage, and WebSocket client code.
- `crates/shared`: Shared message types, admin models, and queue models used by both server and web.

Persistent data defaults to `data/store.json`. The initial super admin is created through the first-run setup form. Keep local credentials out of commits.

## Build, Test, and Development Commands

- `nix develop`: enter the dev shell with Rust, `wasm32-unknown-unknown`, Dioxus CLI, and Binaryen.
- `cargo check --workspace`: type-check all crates.
- `cargo test --workspace`: run workspace unit tests.
- `cargo fmt --all`: format Rust code with rustfmt.
- `cargo clippy --workspace --all-targets`: run lint checks before opening a PR.
- `cargo run -p server`: start the backend on `127.0.0.1:3000`.
- `dx serve --package web`: serve the Dioxus frontend; it expects `ws://127.0.0.1:3000/ws`.

Use `DATA_PATH=/tmp/lue-store.json cargo run -p server` for isolated persistence.

## Coding Style & Naming Conventions

Use Rust 2021 edition and standard rustfmt output. Keep shared protocol types in `shared`, backend-only behavior in `server`, and UI/client behavior in `web`.

Use `snake_case` for functions, variables, modules, and test names; `PascalCase` for types; and `SCREAMING_SNAKE_CASE` for constants. Keep comments brief and reserve them for non-obvious behavior.

## Testing Guidelines

Existing tests are Rust unit tests in modules such as `crates/server/src/store.rs`, `persistence.rs`, and `password.rs`. Add focused tests near the code using `#[cfg(test)] mod tests`.

Run `cargo test --workspace` before submitting changes. For store or persistence changes, include tests for authorization, state transitions, and serialization behavior where applicable.

## Commit & Pull Request Guidelines

Recent git history has no strong local convention. Use concise, imperative commit messages, for example `Add queue persistence test`.

Pull requests should include a short description, verification commands, linked issues when relevant, and screenshots or recordings for visible Dioxus UI changes. Note any setup, `DATA_PATH`, or migration considerations.

## Agent-Specific Instructions

Do not commit generated local state from `data/`, `.nix-home/`, or `target/`. Keep changes scoped to the relevant crate, and update `crates/shared` only when the server and web contract changes.

Keep this guide current. When changing project structure, commands, tests, configuration, or contributor workflow, update `AGENTS.md` in the same change.
