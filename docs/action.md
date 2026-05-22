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

Use the same released tag for `<action-tag>` and `<release-tag>`. Use
`version: latest` only when workflows should track the newest rudolint release
automatically.

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

## Self-Test Workflow

The repository includes a manual `GitHub Action self-test` workflow that runs
the local `./` action against checked-in fixtures. Run it after publishing a
release by passing the release tag as `version`. The workflow keeps the same
contract as user workflows: it downloads a released binary and does not compile
Rust inside the action path.

## Versioning Policy

`action.yml` is released from the same repository and tag as the `rudolint`
binary artifacts. For reproducible CI, pin both the action reference and the
downloaded binary to the same release tag:

```yaml
- uses: kubeply/rudolint@<release-tag>
  with:
    version: <release-tag>
```

Use `version: latest` only when a workflow should automatically pick up newer
`rudolint` releases while keeping the checked-out action implementation pinned.
This can be useful for non-blocking advisory jobs, but blocking CI should prefer
an explicit tag.

Before `1.0.0`, every released tag is treated as the complete action contract
for that release. The project does not publish floating major tags such as `v0`
or `v1` yet; add those only after the action and CLI contracts are stable enough
to support compatibility expectations.

## Marketplace

The action can be used by referencing this repository and a release tag. GitHub
Marketplace publication is tracked separately in milestone 20, because it
requires the publishing account or organization to accept GitHub's Marketplace
Developer Agreement.

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
