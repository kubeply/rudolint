# Output Formats

`rudolint check` supports `text`, `json`, and `sarif` output. The default is
`text`.

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
The schema uses JSON Schema draft 2020-12 and disallows properties that are not
listed below.

### JSON Compatibility Policy

`schemaVersion` names the JSON contract that automation can validate and parse.
After `rudolint` reaches `v1.0.0`, patch and minor releases must keep
`schemaVersion: "v1"` backward compatible.

The following changes are breaking and require a new schema version:

- removing an existing envelope, finding, label, or span field
- renaming a field or changing its JSON type
- changing severity values or rule code format
- changing span coordinate semantics
- making an optional field required
- adding a required field to any existing object

Non-breaking documentation clarifications and bug fixes that keep existing v1
JSON consumers valid may stay on `schemaVersion: "v1"`. Consumers that only
parse JSON can usually ignore extra fields, but strict-schema validators cannot:
the v1 schema sets `additionalProperties: false`. Emitting new fields under
`schemaVersion: "v1"` therefore must not happen unless the project first ships a
coordinated schema evolution that relaxes the schema, notifies consumers, and
allows time for adoption. In normal cases, prefer a new schema version for newly
emitted fields.

Envelope fields:

| Field | Type | Description |
| --- | --- | --- |
| `schemaVersion` | string | Stable output contract version. The v1 schema requires `v1`. |
| `findings` | array | Lint findings emitted by the selected rules and configuration. |

Finding fields:

| Field | Type | Description |
| --- | --- | --- |
| `code` | string | Stable rule identifier such as `DL3007`, `RDK1000`, or `SC2086`. |
| `severity` | string | One of `ignore`, `style`, `info`, `warning`, or `error`. |
| `message` | string | Human-readable diagnostic message. |
| `path` | string | Display path for the Dockerfile that produced the finding. |
| `primary_span` | object | Primary source range for the finding. |
| `labels` | array | Optional secondary source ranges with messages. |
| `help` | string or null | Optional remediation hint. |

Span fields are one-based for line and column positions and zero-based for byte
offsets:

| Field | Type | Description |
| --- | --- | --- |
| `start_byte` | integer | Start byte offset in the source text. |
| `end_byte` | integer | End byte offset in the source text. |
| `start_line` | integer | Start line, beginning at `1`. |
| `start_column` | integer | Start column, beginning at `1`. |
| `end_line` | integer | End line, beginning at `1`. |
| `end_column` | integer | End column, beginning at `1`. |

```json
{
  "schemaVersion": "v1",
  "findings": [
    {
      "code": "DL3007",
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

When `--fix --dry-run --format json` is used, the command emits an envelope
with the same `schemaVersion` and `findings` fields plus a `fixes` array for fix
previews. The committed v1 findings schema describes the `schemaVersion` and
`findings` portion of this envelope; fix preview entries are documented by the
fix preview output itself and may grow independently before v1.

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

## Text

Text output is grouped by file and starts with a compact run summary:

```text
rudolint found 2 findings in 1 file (1 error, 1 warning)

Dockerfile
  ! warning DL3007    1:1   avoid mutable latest base image tags
  x error   DL3000    2:1   WORKDIR should be absolute
```

Output uses ANSI color automatically when stdout is a terminal. Use
`--color always` to force colors or `--color never` to disable them. `NO_COLOR`
and `CLICOLOR=0` disable automatic colors.

`rudolint check --help` includes the finding category legend with the same
severity markers and colors used by text output when colored help is enabled.

`--show-source` appends compact source excerpts to the default output. JSON and
SARIF already carry machine-readable source spans.
