# Hadolint Migration

This guide covers the low-friction path for repositories that already use
Hadolint and want to try `rudolint`.

## Choose A Profile

Start with `hadolint-compat` when replacing an existing Hadolint check. It keeps
Hadolint-style Dockerfile and shell rules enabled while excluding
BuildKit-native recommendations.

```bash
rudolint check . --profile hadolint-compat
```

Use the default profile after the compatibility pass is clean if you want the
additional BuildKit, Buildx, performance, and hardening signal.

```bash
rudolint check .
```

## Migrate Inline Ignores

Hadolint comments such as:

```Dockerfile
# hadolint ignore=DL3008,SC2086
RUN apt-get update && apt-get install -y curl
```

can be converted to `rudolint` comments:

```Dockerfile
# rudolint ignore=DL3008,SC2086
RUN apt-get update && apt-get install -y curl
```

Preview the migration first:

```bash
rudolint check . --fix --migrate-hadolint-ignores --dry-run
```

Apply it when the preview looks correct:

```bash
rudolint check . --fix --migrate-hadolint-ignores
```

The migration only rewrites supported ignore comments. It does not change
Dockerfile instructions or rule behavior.

## Migrate Config

Hadolint commonly uses `.hadolint.yaml` with `ignored` entries:

```yaml
ignored:
  - DL3008
  - SC2086
```

Rudolint accepts `ignored` when reading an existing Hadolint config with
`--config`, but native `.rudolint.yaml` files should use the `ignore` key:

```yaml
ignore:
  - DL3008
  - SC2086
```

Rudolint also supports severity overrides and per-file ignores:

```yaml
severity:
  DL3008: info
per-file-ignores:
  "docs/examples/**/Dockerfile":
    - DL3008
```

## GitHub Action

Replace a Hadolint action with the Rudolint Marketplace action:

```yaml
- uses: kubeply/rudolint@v1
  with:
    profile: hadolint-compat
    failure-threshold: warning
```

The action uses `.rudolint.yaml` by default when it is discovered from the
checked paths. Pass `config` only when the config file lives somewhere else.

## After The First Pass

Once the compatibility profile is stable, run focused profiles to decide which
extra signal is useful for the repository:

```bash
rudolint check . --profile correctness
rudolint check . --profile performance
rudolint check . --profile hardening
```

Use `default` when you want every implemented rule.
