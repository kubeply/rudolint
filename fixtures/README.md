# Fixtures

Fixtures are shared inputs for parser, rule, CLI, compatibility, BuildKit, and
benchmark tests. Keep fixtures small, focused, and named after the behavior they
exercise.

## Layout

- `parser/`: Dockerfile syntax and parser behavior.
- `rules/`: rule-specific diagnostics and fix behavior.
- `cli/`: command-line behavior and end-to-end output.
- `buildkit/`: BuildKit and Buildx syntax or semantic fixtures.
- `compat/`: Hadolint compatibility fixtures and oracle metadata.
- `corpus/`: generated or curated benchmark corpora.

## Naming

- Parser fixtures use `parser/<feature-name>/Dockerfile`.
- Rule fixtures use `rules/<rule-id>.<short-name>/Dockerfile`.
- CLI fixtures use `cli/<behavior-name>/`.
- BuildKit fixtures use `buildkit/<feature-name>/Dockerfile`.
- Compatibility fixtures use `compat/<rule-id>.<short-name>/Dockerfile`.
- Corpus fixtures use `corpus/<case-name>/Dockerfile`, or
  `corpus/<case-name>/` when a benchmark case needs multiple files.

Use lowercase names with hyphens for descriptive parts, for example
`rules/DL3007.no-latest-tag/Dockerfile`,
`buildkit/multi-stage-cache/Dockerfile`, or
`corpus/large-image-scan/Dockerfile`.

## Per-Fixture Files

- `Dockerfile`: primary input.
- `.rudolint.yaml`: optional fixture-specific config.
- `expected.json`: normalized diagnostic expectation when snapshots alone are
  not enough.
- `README.md`: optional context for unusual fixtures.

Snapshots should stay near the test that owns them. Fixture inputs should not
contain generated snapshot output.
