# Crates

`rudolint` follows a small-crate workspace layout so parser, rule engine,
output rendering, and CLI orchestration can evolve independently.

## Layout

- `rudolint-bench` owns benchmark harness support and corpus metadata.
- `rudolint-buildkit` owns BuildKit frontend, mount, entitlement, and Buildx
  semantics.
- `rudolint-cli` packages the `rudolint` binary.
- `rudolint-config` loads and represents configuration.
- `rudolint-diagnostics` defines diagnostics shared across crates.
- `rudolint-dockerfile` parses Dockerfiles into a source-aware syntax model.
- `rudolint-fix` owns autofix edit generation and patch planning.
- `rudolint-image` parses container image references.
- `rudolint-lsp` owns the stdio language server and editor integration
  primitives.
- `rudolint-output` renders diagnostics as human text, JSON, and SARIF.
- `rudolint-policy` owns rule selection and compatibility profiles.
- `rudolint-rules` owns the rule catalog and rule engine.
- `rudolint-settings` resolves effective settings from config and CLI inputs.
- `rudolint-shell` owns shell parsing and `RUN` command analysis.
- `rudolint-source` owns source text, spans, comments, and edit ranges.
- `rudolint-test` owns shared test-only fixture, snapshot, and CLI helpers.
- `xtask` owns repository maintenance commands that should not ship in the
  runtime binary.

## Dependency Direction

```text
rudolint-cli
  -> rudolint-settings
  -> rudolint-config
  -> rudolint-dockerfile
  -> rudolint-rules
  -> rudolint-output

rudolint-rules
  -> rudolint-buildkit
  -> rudolint-config
  -> rudolint-diagnostics
  -> rudolint-dockerfile
  -> rudolint-image
  -> rudolint-policy
  -> rudolint-shell

rudolint-output
  -> rudolint-diagnostics

rudolint-lsp
  -> rudolint-settings
  -> rudolint-rules
  -> rudolint-dockerfile
  -> rudolint-diagnostics
  -> rudolint-fix

rudolint-fix
  -> rudolint-source

test crates and integration tests
  -> rudolint-test

maintenance workflows
  -> xtask
```

Keep `rudolint-source`, `rudolint-diagnostics`, and `rudolint-dockerfile`
dependency-light. Rules may depend on semantic helper crates, but parser,
diagnostics, and source-span crates must not depend on rule or output crates.
`rudolint-test` is dev-only and must not be used by production crate
dependencies.
