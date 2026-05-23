# Configuration

`rudolint` loads project configuration from `.rudolint.yaml` or an explicit
`--config` path.

The v1 configuration schema is committed at
[`schemas/rudolint-config-v1.schema.json`](../schemas/rudolint-config-v1.schema.json).
Schema-aware editors can use it to validate config keys and value shapes before
the linter runs.

## Editor Schema Setup

For a repository-local schema reference, add this comment at the top of
`.rudolint.yaml`:

```yaml
# yaml-language-server: $schema=./schemas/rudolint-config-v1.schema.json
```

For editor settings that support YAML schema mappings, map `.rudolint.yaml` to
the raw schema URL:

```json
{
  "yaml.schemas": {
    "https://raw.githubusercontent.com/kubeply/rudolint/main/schemas/rudolint-config-v1.schema.json": ".rudolint.yaml"
  }
}
```

Use the same schema URL in CI or editor integrations that accept a JSON Schema
URI.

## Precedence

Configuration resolves in this order:

| Priority | Source | Behavior |
| --- | --- | --- |
| 1 | `--no-config` | Disables config loading and discovery. Defaults are used. This flag conflicts with `--config`. |
| 2 | `--config <path>` | Loads the explicit file and skips discovery. |
| 3 | Discovered `.rudolint.yaml` | Walks upward from the input Dockerfile paths or directories and uses the nearest `.rudolint.yaml` found from the first matching start path. |
| 4 | Defaults | Used when no explicit or discovered config is loaded. |

Per-file ignore patterns are matched relative to the loaded config file's
directory. When no config file is loaded, paths are linted as provided.
