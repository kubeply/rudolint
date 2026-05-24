# Fixture Corpus

This directory contains stable Dockerfile inputs used for benchmarks and
regression tests.

## Benchmark Fixtures

These fixtures are stable inputs for startup, single-file, recursive directory,
JSON rendering, and SARIF rendering benchmarks.

- `small/`: one small Dockerfile.
- `medium-multistage/`: one realistic multi-stage Dockerfile.
- `large-generated/`: one generated large Dockerfile.
- `directory-tree/`: a small recursive tree with multiple Dockerfiles.
- `buildkit-heavy/`: one Dockerfile that exercises BuildKit-heavy parsing and
  rule paths.

Benchmark results should record the command, commit SHA, OS, target triple, and
hardware notes.

## Real-World Regression Fixtures

`real-world/` contains curated Dockerfiles that protect parser and lint behavior
for common production patterns. Each fixture directory contains:

- `Dockerfile`: the regression input.
- `metadata.md`: a short explanation of why the fixture exists and which
  behavior it protects.

The real-world corpus intentionally includes both positive fixtures that produce
stable findings and noise fixtures that should not produce findings. Noise
fixture directories use the `noise-` prefix and are covered by a dedicated test.

## Update Guidelines

When adding or changing corpus fixtures:

1. Keep fixtures deterministic. They must be readable without network access,
   Docker, BuildKit, package managers, or external repositories.
2. Prefer realistic Dockerfile syntax over minimized rule examples. Focused rule
   examples belong in `fixtures/rules/`.
3. Add or update `metadata.md` with the behavior the fixture protects.
4. If the fixture should produce findings, update the lint snapshot deliberately.
5. If the fixture should stay quiet, add it to the noise fixture test instead of
   relying on manual inspection.
6. Update parser snapshots whenever Dockerfile syntax coverage changes.
7. Keep benchmark fixtures stable. Changes to benchmark inputs can invalidate
   performance history and should be called out in the pull request.
8. Do not add generated dependency directories, vendored source trees, archives,
   or files that require manual cleanup.

Run the relevant package tests with snapshot updates disabled before opening a
pull request:

```console
INSTA_UPDATE=no cargo test -p rudolint-dockerfile --locked
INSTA_UPDATE=no cargo test -p rudolint-rules --locked
```
