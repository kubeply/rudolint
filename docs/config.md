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
