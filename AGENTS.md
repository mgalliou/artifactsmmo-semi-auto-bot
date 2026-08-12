# Repository Guidance

## Workspace

- This is a Rust 2024 workspace with seven members. Root Cargo commands target the default members `repl` and `tui`; use `--workspace` when validating all workspace crates.
- Dependency flow is `openapi` -> `api` -> `sdk` -> `bot` -> the `repl` and `tui` frontends; `sdk_derive` is the proc-macro crate used by `sdk`.
- `repl` and `tui` initialize the SDK client and start `bot`; business/API behavior belongs in the lower crates rather than the frontends.

## Verification

- Full workspace: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`.
- Focus a crate or test with `cargo test -p <crate>` or `cargo test -p <crate> <test_name>`.
- `bot` tests consume `sdk`'s `test-utils` feature through a dev-dependency. SDK-backed tests read committed RON data from `crates/sdk/tests/fixtures`; update those fixtures when test catalog data must change.
- The rate-limiter unit tests intentionally wait on real timing intervals, so `api` tests are slower and timing-sensitive.

## Runtime

- Run the REPL from the repository root with `cargo run -p repl`; relative config and cache paths assume that working directory.
- Frontends read the API token from `ARTIFACTSMMO_TOKEN`, use the production API URL, and cache SDK catalog data as RON files under `.cache/`. `Client::new` discovers the account name from the authenticated account-details endpoint; do not add separate account-name configuration.
- `Bot::new` unconditionally parses `ArtifactsMMO.toml` from the current directory and panics if it is missing or invalid. It contains positional bot/character behavior settings, is gitignored, and may contain local secrets; the current schema is documented in `README.md`.
- Starting a frontend launches live bot and character threads that can issue game actions; do not use a run command as routine verification.

## Generated Client

- Treat `crates/openapi/src` and its generated manifest content as generated code. Make API-shape changes through regeneration, not hand edits.
- Regenerate from inside `crates/openapi` with `./gen.sh`; the script uses `npx`, downloads the live ArtifactsMMO schema, deletes `src/`, and patches the generated `Cargo.toml` for this workspace. OpenAPI Generator is pinned in `openapitools.json`.
- After regeneration, review both `crates/openapi/src` and `crates/openapi/Cargo.toml`, then run full-workspace verification because downstream wrappers depend directly on generated models and APIs.
