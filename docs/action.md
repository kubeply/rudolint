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
      - uses: kubeply/rudolint@<action-tag>
        with:
          version: <release-tag>
```

Use the same released tag for `<action-tag>` and `<release-tag>` after the
first release is published. Use `version: latest` only when workflows should
track the newest rudolint release automatically.

## Monorepo Paths

`paths` accepts one path per line:

```yaml
- uses: kubeply/rudolint@<action-tag>
  with:
    version: <release-tag>
    paths: |
      services/api
      services/worker/Dockerfile
```

## Compatibility Profile

```yaml
- uses: kubeply/rudolint@<action-tag>
  with:
    version: <release-tag>
    profile: compat
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
      - uses: kubeply/rudolint@<action-tag>
        with:
          version: <release-tag>
          upload-sarif: "true"
          sarif-output: rudolint.sarif
```

## Inputs

- `version`: release tag to install, or `latest`.
- `repository`: repository that publishes rudolint release artifacts.
- `paths`: newline-separated Dockerfile paths or directories.
- `config`: optional `.rudolint.yaml` path.
- `profile`: `default` or `compat`.
- `format`: `human`, `json`, or `sarif` when `upload-sarif` is false.
- `failure-threshold`: `ignore`, `style`, `info`, `warning`, or `error`.
- `sarif-output`: SARIF path to write when `upload-sarif` is true.
- `upload-sarif`: upload SARIF through GitHub code scanning.
- `github-token`: optional token for release downloads.

## Outputs

- `exit-code`: `rudolint` process exit code.
- `findings-count`: number of findings when the action can count the selected
  output format.
- `sarif-path`: SARIF path when SARIF output is written.
