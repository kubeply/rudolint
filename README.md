# rudolint

[![CI](https://github.com/kubeply/rudolint/actions/workflows/ci.yml/badge.svg)](https://github.com/kubeply/rudolint/actions/workflows/ci.yml)
[![Release](https://github.com/kubeply/rudolint/actions/workflows/release.yml/badge.svg)](https://github.com/kubeply/rudolint/actions/workflows/release.yml)
[![GitHub release](https://img.shields.io/github/v/release/kubeply/rudolint?sort=semver)](https://github.com/kubeply/rudolint/releases)
[![Rust 1.95.0](https://img.shields.io/badge/rust-1.95.0-orange)](rust-toolchain.toml)
[![License](https://img.shields.io/github/license/kubeply/rudolint)](LICENSE)
[![GitHub Marketplace](https://img.shields.io/badge/marketplace-rudolint-blue?logo=github)](https://github.com/marketplace/actions/rudolint)
[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/kubeply/rudolint?utm_source=badge)

`rudolint` is a fast Dockerfile linter built for modern BuildKit and Buildx
workflows.

![Dockerfile linter performance](benchmarks/dockerfile-linters/results/headline.svg)

## Features

| Feature | What is implemented |
| --- | --- |
| Dockerfile parsing | Source-aware parsing for instructions, flags, heredocs, comments, and stage aliases. |
| BuildKit support | Frontend syntax directives, cache mounts, secret mounts, SSH mounts, insecure entitlements, and Buildx platform checks. |
| Rule profiles | `default` runs Hadolint-style compatibility rules plus BuildKit-native rules. `hadolint-compat` runs Hadolint-style and shell-style rules without BuildKit-native `RDK` diagnostics. |
| CI output | Human output for terminals, JSON for automation, and SARIF for GitHub code scanning. |
| Configuration | `.rudolint.yaml` supports ignored rules, severity overrides, per-file ignores, selected rule prefixes, and trusted registries. |
| GitHub Action | A [Marketplace action](https://github.com/marketplace/actions/rudolint) downloads checksummed release binaries and runs `rudolint` without compiling Rust in user workflows. |
| Language server | `rudolint-lsp` speaks LSP over stdio for editor diagnostics. |
| Rule documentation | Implemented rules are documented under [docs/rules](docs/rules/README.md), with compatibility and BuildKit roadmap notes in [docs/rule-roadmap.md](docs/rule-roadmap.md). |

## Install

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/kubeply/rudolint/releases/latest/download/rudolint-installer.sh | sh
```

## Usage

Check one Dockerfile:

```bash
rudolint check Dockerfile
```

Check a repository:

```bash
rudolint check .
```

Write SARIF for CI:

```bash
rudolint check . --format sarif > rudolint.sarif
```

Exit codes:

| Code | Meaning |
| --- | --- |
| `0` | No findings at or above the failure threshold. |
| `1` | Findings met or exceeded the failure threshold. |
| `2` | Usage, config, or input error. |
| `3` | Unexpected internal error. |

## GitHub Action

Use the stable major action tag for normal CI. By default, the action downloads
the latest released `rudolint` binary, so the wrapper and linter stay aligned
across stable releases:

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

Set `version` only when you want to pin a specific linter release:

```yaml
- uses: kubeply/rudolint@v1
  with:
    version: <release-tag>
```

For monorepos, pass one path per line:

```yaml
- uses: kubeply/rudolint@v1
  with:
    paths: |
      services/api
      services/worker/Dockerfile
```

Upload SARIF to GitHub code scanning:

```yaml
permissions:
  contents: read
  security-events: write

steps:
  - uses: actions/checkout@v4
  - uses: kubeply/rudolint@v1
    with:
      upload-sarif: "true"
      sarif-output: rudolint.sarif
```

Use `profile: hadolint-compat` when you want Hadolint-style Dockerfile and
shell-style checks without BuildKit-native `RDK` diagnostics. This is useful
while migrating from Hadolint or when a project is not ready for BuildKit
recommendations yet. See [docs/action.md](docs/action.md) for the full action
contract.

```yaml
- uses: kubeply/rudolint@v1
  with:
    profile: hadolint-compat
    failure-threshold: error
```

## Configuration

`.rudolint.yaml` is documented in [docs/config.md](docs/config.md), including
the v1 JSON Schema for editor validation.

```yaml
ignore:
  - RDK1003

select:
  - DL
  - RDK1008

severity:
  DL3007: error
  RDK1000: warning

per-file-ignores:
  # Patterns are matched relative to this config file's directory.
  fixtures/**:
    - DL3000

trusted-registries:
  - docker.io
  - ghcr.io
```

Run with:

```bash
rudolint check . --config .rudolint.yaml
```

## Rule Families

| Prefix | Scope |
| --- | --- |
| `DL` | Hadolint-compatible Dockerfile diagnostics. |
| `RDK` | Rudolint-native BuildKit diagnostics such as frontend directives, mounts, entitlements, and Buildx platform behavior. |
| `RUD` | Rudolint-native migration and project policy diagnostics. |
| `SC` | ShellCheck-compatible diagnostics extracted from `RUN` bodies. |

## More Docs

- [GitHub Action usage](docs/action.md)
- [Configuration](docs/config.md)
- [Rule roadmap](docs/rule-roadmap.md)
- [Architecture](docs/architecture.md)
- [Completed implementation plan](docs/archive/implementation-plan.md)
- [Release automation](docs/release.md)

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, repository layout,
validation, testing, and contribution guidelines.
