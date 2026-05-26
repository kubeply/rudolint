# GitHub Action

The `rudolint` action is a thin wrapper around released binaries. It downloads a
checksummed release artifact, runs `rudolint check`, and optionally uploads SARIF
to GitHub code scanning. It does not compile Rust in user workflows.

## Basic Check

```yaml
name: Dockerfile lint

on:
  pull_request:
  push:
    branches: [main]

jobs:
  rudolint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: kubeply/rudolint@v1
```

The `v1` action tag tracks the latest stable `v1.x.y` action wrapper. The
`version` input defaults to `latest`, so normal workflows also download the
latest released `rudolint` binary.

Set `version` only when a workflow needs a specific linter release:

```yaml
- uses: kubeply/rudolint@v1
  with:
    version: <release-tag>
```

## Monorepo Paths

`paths` accepts one path per line:

```yaml
- uses: kubeply/rudolint@v1
  with:
    paths: |
      services/api
      services/worker/Dockerfile
```

## Rule Profiles

The default profile runs all implemented rules, including shell-style checks
and BuildKit-native diagnostics. Use `hadolint-compat` when a workflow should
stay on Hadolint-style Dockerfile and shell-style checks while excluding
BuildKit-native recommendations. Use `correctness`, `performance`, or
`hardening` when a workflow should focus on one signal category.

```yaml
- uses: kubeply/rudolint@v1
  with:
    profile: hadolint-compat
    failure-threshold: error
```

## SARIF Upload

SARIF upload requires `security-events: write`.

```yaml
permissions:
  contents: read
  security-events: write

jobs:
  rudolint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: kubeply/rudolint@v1
        with:
          upload-sarif: "true"
          sarif-output: rudolint.sarif
```

## Self-Test Workflow

The repository includes a manual `GitHub Action self-test` workflow that runs
the local `./` action against checked-in fixtures. Run it after publishing a
release by passing the release tag as `version`. The workflow keeps the same
contract as user workflows: it downloads a released binary and does not compile
Rust inside the action path.

## Versioning Policy

`action.yml` is released from the same repository as the `rudolint` binary
artifacts. The recommended default is the stable major action tag:

```yaml
- uses: kubeply/rudolint@v1
```

This keeps most workflows on the latest stable action wrapper and latest
released linter binary. Pin `version` only when the linter binary must stay on
a specific release:

```yaml
- uses: kubeply/rudolint@v1
  with:
    version: <release-tag>
```

For fully reproducible CI, pin both the action reference and downloaded binary
to the same release tag:

```yaml
- uses: kubeply/rudolint@<release-tag>
  with:
    version: <release-tag>
```

The `v1` tag points to the latest stable `v1.x.y` release. It must not point to
prerelease tags such as `v1.1.0-rc.1`, `v1.1.0-beta.1`, or any tag with a
SemVer prerelease suffix. Move `v1` only after the stable release workflow has
finished and post-release verification has passed.

## Marketplace

The action is published on
[GitHub Marketplace](https://github.com/marketplace/actions/rudolint).
Marketplace examples should use the common `v1` form:

```yaml
- uses: kubeply/rudolint@v1
```

Show `version` only when documenting how to pin a specific linter release.

## Inputs

- `version` (optional): release tag to install. Set this only to pin a
  specific linter release. Default: `latest`.
- `repository` (optional): repository in `owner/repo` format that publishes
  rudolint release artifacts. Default: `kubeply/rudolint`.
- `paths` (optional): newline-separated Dockerfile paths or directories.
  Default: `.`.
- `config` (optional): `.rudolint.yaml` path. Default: empty, which lets
  rudolint use its normal config discovery.
- `profile` (optional): `default`, `hadolint-compat`, `correctness`,
  `performance`, or `hardening`. Default: `default`.
- `format` (optional): `text`, `json`, or `sarif` when `upload-sarif` is
  false. Default: `text`.
- `failure-threshold` (optional): `ignore`, `style`, `info`, `warning`, or
  `error`. Default: `warning`.
- `sarif-output` (optional): SARIF path to write when `upload-sarif` is true.
  Default: `rudolint.sarif`.
- `upload-sarif` (optional): upload SARIF through GitHub code scanning.
  Default: `false`.
- `github-token` (optional): token for release downloads. Useful to avoid rate
  limits or to access a private `repository`. Default: empty.

## Outputs

- `exit-code`: `rudolint` process exit code.
- `findings-count`: number of findings when the action can count the selected
  output format.
- `sarif-path`: SARIF path when SARIF output is written.
