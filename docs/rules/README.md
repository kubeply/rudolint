# Rule Documentation

Each implemented rule should have a dedicated page named after the rule code,
for example `RDL3007.md` or `RDK1000.md`.

Rule pages should include:

- rule code and title.
- default severity.
- rule family and category.
- provenance.
- rationale.
- bad example.
- good example.
- configuration notes.
- autofix behavior:
  - safe automatic fix.
  - manual suggestion.
  - no-fix rationale.
- compatibility notes.

`RDL` pages document Hadolint compatibility provenance. `RDK` pages document
project-native BuildKit behavior. `RSC` pages document shell-analysis behavior
implemented by `rudolint-shell`.

## Adding a Rule

A new rule is ready to land when it has:

- stable metadata in the rule catalog, including code, name, summary, default
  severity, profile family, category, status, docs URL, and autofix
  availability.
- a docs page in this directory named `<RULE_ID>.md`.
- at least one positive fixture or snapshot proving the rule reports the
  intended finding.
- a negative or noise fixture when the rule has a meaningful false-positive
  risk.
- source-span coverage for emitted findings.
- an explicit autofix status of `safe`, `manual`, `not-applicable`, or
  `not-yet`.
- an entry in [`../rule-roadmap.md`](../rule-roadmap.md) under the implemented
  matrix when the rule is active, or under the planned section when the rule is
  only tracked for future work.

Compatibility rules should preserve Hadolint behavior unless the docs call out
an intentional divergence. BuildKit-native rules should explain the BuildKit
feature or failure mode they protect. Shell rules should come from the
shell-analysis layer rather than ad hoc Dockerfile substring checks.
