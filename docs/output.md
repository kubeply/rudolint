# Output Formats

`rudolint check` supports `human`, `json`, and `sarif` output. The default is
`human`.

## Stability

The JSON schema is review-stabilized with snapshots, but it may change before
`rudolint` reaches `1.0.0`. Treat JSON fields as public enough for CI scripts to
consume, but pin the binary version when depending on exact field names.

SARIF output targets SARIF 2.1.0 and GitHub code scanning upload expectations.
Changes should preserve the top-level `$schema`, `version`, `runs`, tool driver,
rule metadata, result locations, and physical source regions.

## JSON

`rudolint check --format json` emits a versioned JSON envelope. This output is
the v1 findings schema surface and is described by
[`schemas/rudolint-findings-v1.schema.json`](../schemas/rudolint-findings-v1.schema.json).

```json
{
  "schemaVersion": "v1",
  "findings": [
    {
      "code": "RDL3007",
      "message": "avoid using latest tag",
      "severity": "warning",
      "path": "Dockerfile",
      "primary_span": {
        "start_line": 1,
        "start_column": 1,
        "end_line": 1,
        "end_column": 19,
        "start_byte": 0,
        "end_byte": 18
      },
      "labels": [],
      "help": null
    }
  ]
}
```

When `--fix --dry-run --format json` is used, the command emits an envelope:

```json
{
  "schemaVersion": "v1",
  "findings": [],
  "fixes": []
}
```

## SARIF

`rudolint check --format sarif` emits a SARIF 2.1.0 report with one run. Each
result includes:

- `ruleId`
- SARIF severity `level`
- message text
- artifact URI
- source region with start and end line/column

The tool driver includes `rudolint` metadata and rule entries for every emitted
finding code.

## Human

Human output is grouped by file:

```text
Dockerfile:
  1:1 warning RDL3007 avoid using latest tag
```

`--show-source` appends compact source excerpts to human output only. JSON and
SARIF already carry machine-readable source spans.
