# Architecture

`rudolint` is split into focused crates. The crate boundaries should stay
small and boring so each part can be tested independently.

## Workspace Crates

- `rudolint`: binary crate and CLI orchestration.
- `rudolint-bench`: benchmark harness support and corpus metadata.
- `rudolint-buildkit`: BuildKit frontend, mount, entitlement, and Buildx
  semantics.
- `rudolint-config`: configuration loading and config-domain types.
- `rudolint-diagnostics`: diagnostic records, severities, and source paths.
- `rudolint-dockerfile`: Dockerfile parser and syntax model.
- `rudolint-fix`: autofix edit generation and patch planning.
- `rudolint-image`: container image reference parsing.
- `rudolint-lsp`: stdio language server and editor integration primitives.
- `rudolint-output`: human, JSON, and SARIF renderers.
- `rudolint-policy`: rule selection, profiles, and compatibility policy.
- `rudolint-rules`: rule catalog and rule engine.
- `rudolint-settings`: resolved settings after config and CLI overrides.
- `rudolint-shell`: shell parsing and `RUN` command analysis.
- `rudolint-source`: source text, spans, comments, and edit ranges.
- `rudolint-test`: shared test-only fixture, snapshot, normalization, and CLI
  helpers.
- `xtask`: repository maintenance commands that do not ship in the runtime
  binary.

## Parser

`rudolint-dockerfile` owns source spans and Dockerfile syntax. It should
understand:

- parser directives
- instruction continuations
- flags on `FROM`, `RUN`, `COPY`, and `ADD`
- BuildKit mounts
- heredocs
- stage aliases

The parser should not decide whether code is good or bad.

## Model

The model turns parsed instructions into facts rules can consume:

- stages
- base images
- copied files
- `RUN` commands
- BuildKit frontend version
- mount graph
- declared build arguments
- environment variables

This layer is intentionally thin at the baseline stage.

## Rules

`rudolint-rules` consumes the model and produces diagnostics. Rules must be
deterministic and must not perform network or filesystem access unless the CLI
explicitly enables repository-wide analysis.

Compatibility rules live under `RDL` and `RSC`. BuildKit-native rules live under
`RDK`.

## Output

`rudolint-output` converts diagnostics into human text, JSON, SARIF, and future
editor/LSP responses. Renderers must not change rule behavior. Format stability
and schema notes live in [`docs/output.md`](output.md).

## Tests

`rudolint-test` owns shared test helpers. It is a dev-only crate for fixture
paths, snapshot normalization, CLI invocation helpers, oracle normalization,
and fix-preview rendering. Production crates must not depend on it.

## Maintenance

`xtask` owns repository maintenance workflows such as refreshing compatibility
oracle metadata. It is a workspace utility and must not be part of the
`rudolint` runtime path.
