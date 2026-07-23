# AGENTS.md

## Purpose

`rudolint` is a BuildKit-native Dockerfile linter written in Rust. Keep the
runtime as a single binary: no shell, Python, Node, Docker, or external linter
dependency in the normal `rudolint check` path.

## Working Rules

- Read `CONTRIBUTING.md`, `crates/README.md`, and `docs/architecture.md` before
  making structural changes.
- Keep workspace crates focused. If a change touches multiple crates, the
  dependency direction should still be CLI -> output/rules/config -> parser and
  diagnostics, not the reverse.
- Placeholder crates are intentional. Put new implementation in the matching
  crate instead of expanding `rudolint-cli` or `rudolint-rules` by default.
- Keep parsing, diagnostics, and rule evaluation separated.
- Treat compatibility tests as oracle tests, not as runtime behavior.
- Prefer source-span preserving parsers and deterministic diagnostics.
- Do not copy third-party rule descriptions wholesale into source or docs.
- Add new rules with focused fixtures and JSON/SARIF output coverage.
- Always attempt to add a test case for changed behavior.
- Prefer integration tests under each crate's `tests/` directory for CLI and
  rule behavior. Use unit tests for parser/model edge cases that are awkward to
  express through the CLI.
- Prefer `insta` snapshots for structured diagnostics and SARIF/JSON output
  instead of broad substring assertions.
- Read nearby tests before adding new cases and keep the style consistent.
- Avoid `panic!`, `unreachable!`, `.unwrap()`, `unsafe`, and clippy ignores in
  production code. In tests, use them sparingly when the failure would be
  clearer than manual error plumbing.
- Prefer fallible control flow such as `if let`, `let ... else`, and `Result`
  propagation over assuming success.
- Prefer let chains over nested `if let` statements when they make the code
  easier to read.
- If `unsafe` is ever required, include a `SAFETY:` comment explaining the
  invariant being upheld.
- Prefer `#[expect(...)]` over `#[allow(...)]` when a lint must be suppressed,
  and keep the reason local and specific.
- Never assume clippy warnings are pre-existing; keep `main` warning-free.
- Prefer top-level imports over local imports or fully qualified names when a
  type or function is used more than once.
- Avoid shortened variable names. Use names like `instruction`, `severity`, and
  `frontend_version` instead of abbreviations.
- Prefer [`TypeName`] references in Rust doc comments for public APIs.
- Never run release builds unless the task is explicitly about release
  packaging or performance measurement.
- Prefer running a specific test during iteration, then run the full validation
  set before finishing.
- Never update the whole lockfile casually. If a dependency must change, keep
  the lockfile diff scoped and prefer `cargo update --package <name> --precise
  <version>`.

## Validation

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo hawk check --manifest-path Cargo.toml -D warnings -D hawk::unnecessary_crate_visibility
```
