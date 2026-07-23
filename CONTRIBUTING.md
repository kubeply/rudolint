# Contributing

## Scope

`rudolint` is early-stage. Please discuss large feature work before opening a
pull request. Small parser fixes, rule fixtures, diagnostics improvements, and
documentation fixes are welcome.

## Setup

Install Rust through `rustup`. Until `1.0.0`, `rust-toolchain.toml` is the
exact project toolchain, not a minimum supported Rust version. It includes
`clippy` and `rustfmt`.

```bash
rustup show
cargo --version
```

Install the pinned Hawk 0.1.9 binaries:

```bash
./scripts/install-hawk.sh
```

## Repository Layout

The Rust workspace lives under `crates/`. Each crate should have one clear job.
Read [crates/README.md](crates/README.md) and
[docs/architecture.md](docs/architecture.md) before adding new modules.

Workspace crates:

- `rudolint-cli`: binary crate and CLI orchestration.
- `rudolint-bench`: benchmark harness support and corpus metadata.
- `rudolint-buildkit`: BuildKit and Buildx semantic analysis.
- `rudolint-config`: config loading and config-domain types.
- `rudolint-diagnostics`: severities and diagnostic records.
- `rudolint-dockerfile`: Dockerfile parser and syntax model.
- `rudolint-fix`: autofix edit planning.
- `rudolint-image`: container image reference parsing.
- `rudolint-lsp`: stdio language server and editor integration primitives.
- `rudolint-output`: human, JSON, and SARIF renderers.
- `rudolint-policy`: rule selection and compatibility profiles.
- `rudolint-rules`: rule catalog and rule engine.
- `rudolint-settings`: resolved settings.
- `rudolint-shell`: shell and `RUN` command analysis.
- `rudolint-source`: source text and spans.
- `rudolint-test`: shared test-only fixture, snapshot, and CLI helpers.
- `xtask`: repository maintenance commands that should not ship in runtime
  binaries.

## Validation

Run the same checks as CI before sending a pull request:

```bash
cargo fmt --all --check
cargo check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo deny check
cargo hawk check --manifest-path Cargo.toml -D warnings -D hawk::unnecessary_crate_visibility
```

Hawk treats the released `rudolint` and `rudolint-lsp` binaries as the
production roots for workspace-wide visibility analysis. The additional
`hawk::unnecessary_crate_visibility` denial opts into its strictest supported
visibility reduction.

Once snapshots are added, CI and local validation should run them with updates
disabled:

```bash
INSTA_UPDATE=no cargo test --all-targets --all-features --locked
```

During iteration, prefer targeted tests:

```bash
cargo test -p rudolint-dockerfile parser
cargo test -p rudolint --test cli
```

Run ignored oracle tests only when the pinned oracle binary is installed:

```bash
cargo test --all-targets --all-features --locked -- --ignored
```

## Tests

- Add tests for behavior changes.
- Prefer integration tests for CLI and rule output.
- Prefer `insta` snapshots for structured JSON and SARIF diagnostics.
- Keep external oracle tests ignored unless the required binary is explicitly
  installed through the oracle update workflow.

## Rules

Rules should be deterministic and should not perform network access. Avoid
filesystem access inside rule implementations unless the CLI explicitly enables
repository-wide analysis for that mode.

`DL` rules document Hadolint compatibility provenance. `SC` rules document
shell-analysis behavior. BuildKit-native rules use `RDK`.

Every rule must declare one of:

- tested safe automatic fix.
- tested manual suggestion.
- tested no-fix rationale when a correct edit cannot be inferred.

## Dependencies

Keep dependency additions small and justified. Do not update the full lockfile
unless the change requires it. Prefer scoped updates:

```bash
cargo update --package <name> --precise <version>
```

## Releases

Do not cut releases from local machines. Release automation produces
checksummed artifacts from GitHub Actions through the generated `cargo-dist`
workflow.
