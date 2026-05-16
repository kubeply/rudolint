# rudolint-test

Shared test-only helpers for fixture paths, snapshot normalization, CLI test
setup, oracle normalization, and fix-preview rendering.

This crate must stay dev-only. Production crates should not depend on it.

Initial helpers cover:

- resolving fixture paths from the workspace root.
- reading fixture files.
- constructing the `rudolint` integration-test command.
- normalizing workspace-local paths in JSON output.
- generating stable snapshot names.
