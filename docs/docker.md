# Docker Image

`rudolint` publishes a multi-architecture container image to GitHub Container
Registry for users who do not want to install the CLI directly.

## Quick Start

Run `rudolint` against the current repository:

```bash
docker run --rm -v "$PWD:/workspace" ghcr.io/kubeply/rudolint check /workspace
```

The image uses `rudolint` as its entrypoint and sets `/workspace` as the working
directory. Arguments after the image name are passed directly to `rudolint`.

Check a single Dockerfile:

```bash
docker run --rm -v "$PWD:/workspace" ghcr.io/kubeply/rudolint check /workspace/Dockerfile
```

Use a config file from the mounted repository:

```bash
docker run --rm -v "$PWD:/workspace" ghcr.io/kubeply/rudolint check /workspace --config /workspace/.rudolint.yaml
```

Write SARIF to the host workspace:

```bash
docker run --rm -v "$PWD:/workspace" ghcr.io/kubeply/rudolint check /workspace --format sarif > rudolint.sarif
```

## Tags

Stable releases publish these tags:

- `ghcr.io/kubeply/rudolint:<release-tag>`, for example `v1.3.1`.
- `ghcr.io/kubeply/rudolint:v1`, the latest stable `v1.x.y` release.
- `ghcr.io/kubeply/rudolint:latest`, the latest stable release.

Use exact release tags for reproducible CI. Use `v1` or `latest` when you want
automatic patch and minor updates.

## Platforms

The image is published for:

- `linux/amd64`
- `linux/arm64`

Docker selects the matching platform automatically. To force one explicitly:

```bash
docker run --rm --platform linux/amd64 -v "$PWD:/workspace" ghcr.io/kubeply/rudolint check /workspace
```

## CI Example

```yaml
name: Dockerfile lint

on:
  pull_request:

jobs:
  rudolint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: |
          docker run --rm \
            -v "$PWD:/workspace" \
            ghcr.io/kubeply/rudolint:v1 \
            check /workspace
```

Prefer the [GitHub Action](action.md) for normal GitHub workflows because it
integrates SARIF upload and release-binary checksum verification. The container
image is useful for local runs, generic CI systems, and environments where
Docker is already the standard execution boundary.
