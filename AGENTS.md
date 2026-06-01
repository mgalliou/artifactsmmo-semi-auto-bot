# Agent Guidelines for artifactsmmo-semi-auto-bot

## Project Overview

Rust Cargo workspace (Edition 2024) for a semi-autonomous bot for the ArtifactsMMO game.
Contains 6 crates: `openapi`, `api`, `sdk`, `sdk_derive`, `bot`, `repl`.

## Build, Lint, and Test Commands

### Standard Commands

- `cargo build` - Build all workspace crates
- `cargo build --release` - Release build
- `cargo check` - Type-check without full compilation
- `cargo fmt` - Format code
- `cargo clippy` / `cargo clippy --fix` - Run/fix linter
- `cargo test <test_name>` - Run specific test
- `cargo test -p <package>` - Test specific package

### OpenAPI Client

- `./crates/openapi/update_spec.sh` - Fetch latest API spec
- `./crates/openapi/gen.sh` - Regenerate Rust client

## Code Style

- **Format**: rustfmt.toml Edition 2024 style, run `cargo fmt` before committing
- **Naming**: snake_case (modules/vars), PascalCase (types/enums), SCREAMING_SNAKE_CASE (consts)
- **Imports**: Group std imports separately; use `crate::` for internal
- **Errors**: Use `thiserror` with `#[derive(Debug, Error)]`, lowercase error messages
- **Enums**: Use `strum_macros` with `#[strum(serialize_all = "snake_case")]`
- **Lints**: `pedantic` + `nursery` at warn level (missing_panics_doc/errors_doc allowed)

## Architecture

- **openapi**: Auto-generated (do not edit)
- **api**: API wrapper with pagination
- **sdk**: Core SDK with client, entities, simulator, traits
- **sdk_derive**: `CollectionClient` derive macro
- **bot**: Bot logic, character control, gear finding
- **repl**: CLI interface

## Key Traits

- `Code` - `code(&self) -> &str`
- `Quantity` - `quantity(&self) -> u32`
- `CollectionClient` - derive for data collections
- `Persist<D>` - auto-load/persist JSON
