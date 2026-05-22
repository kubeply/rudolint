# First Release Readiness

This page records the minimum criteria used for the first usable `rudolint`
release. It is a release checklist, not a complete project roadmap. The first
release can ship before every planned rule exists, as long as the shipped
behavior is clear, tested, and easy to install in CI.

`v0.1.0` satisfied this checklist. Future releases should continue to preserve
these guarantees unless a replacement contract is documented first.

## Readiness Checklist

- [x] Parser handles common Dockerfiles, BuildKit mounts, heredocs, stage
  aliases, and platform flags.
  - Covered by parser fixtures under `fixtures/parser/` and snapshots in
    `crates/rudolint-dockerfile/tests/`.
- [x] JSON and SARIF schemas are stable enough for CI use.
  - Documented in [`docs/output.md`](output.md) and snapshot-tested through CLI
    integration tests.
- [x] CLI exit codes are documented and tested.
  - Documented in [`README.md`](../README.md). Tested in
    `crates/rudolint-cli/tests/cli.rs`.
- [x] Config supports ignore lists, severity overrides, trusted registries, and
  BuildKit entitlement opt-ins.
  - Documented in [`README.md`](../README.md). Covered by config, settings, CLI,
    and rule fixture tests.
- [x] Existing compatibility rules are fixture-backed.
  - Tracked in [`docs/rule-roadmap.md`](rule-roadmap.md), with Dockerfile
    fixtures under `fixtures/rules/` and JSON snapshots in
    `crates/rudolint-rules/tests/`.
- [x] `RDK1000` through `RDK1003` use typed parser, BuildKit, shell, and package
  manager facts instead of ad hoc Dockerfile substring checks.
  - Covered by BuildKit and rule fixture snapshots.
- [x] Release automation can produce checksummed Linux and macOS binaries.
  - Configured through `cargo-dist` in [`dist-workspace.toml`](../dist-workspace.toml)
    and `.github/workflows/release.yml`.
- [x] GitHub Action can run without Rust installed in user workflows.
  - The action downloads released binaries and is documented in
    [`docs/action.md`](action.md).
- [x] CI gates formatting, clippy, tests, dependency policy, snapshots, and
  workflow syntax.
  - The default CI workflow runs `cargo fmt`, `cargo check`, `cargo clippy`,
    `cargo test` with `INSTA_UPDATE=no`, `cargo deny`, and `actionlint`.
- [x] Known limitations are documented below.

## Release Procedure

For each release:

1. Confirm `main` is green for CI and the release plan job.
2. Run `dist plan --output-format=json` locally or inspect the latest release
   workflow plan output.
3. Push a semantic version tag from the reviewed `main` commit.
4. Confirm the release workflow publishes archives, checksums, source tarball,
   shell installer, and attestations.
5. Run the manual `GitHub Action self-test` workflow with the new tag.
6. Verify installation with:

```bash
rudolint --version
rudolint check --format json --failure-threshold error < Dockerfile
```

## Known Limitations

- No Windows binary is published yet. Windows support is deferred until path
  handling and shell behavior are tested on Windows CI.
- No Docker image is published yet. Prefer release archives and the GitHub
  Action until there is a concrete CI use case for image distribution.
- JSON output is review-stabilized but may still change before `1.0.0`. Pin the
  binary version when CI depends on exact field names.
- Rule coverage is broad but not complete parity with established Dockerfile
  linters. The implemented rule set is the supported surface for the first
  release.
- Compatibility oracle tests are still manual. Runtime linting is native and
  never shells out to Hadolint or another external linter.
- Buildx Bake files are out of scope for the first release. Dockerfile linting
  remains the supported entrypoint.
- The GitHub Action should be pinned to a release tag for blocking CI. Floating
  major tags such as `v0` or `v1` are not published yet.
- The GitHub Action is not listed on GitHub Marketplace yet. That visibility
  work is tracked separately in milestone 20.
