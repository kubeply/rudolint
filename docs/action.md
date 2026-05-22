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

- `version` (optional): release tag to install, or `latest`. Default:
  `latest`.
- `repository` (optional): repository in `owner/repo` format that publishes
  rudolint release artifacts. Default: `kubeply/rudolint`.
- `paths` (optional): newline-separated Dockerfile paths or directories.
  Default: `.`.
- `config` (optional): `.rudolint.yaml` path. Default: empty, which lets
  rudolint use its normal config discovery.
- `profile` (optional): `default` or `compat`. Default: `default`.
- `format` (optional): `human`, `json`, or `sarif` when `upload-sarif` is
  false. Default: `human`.
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
