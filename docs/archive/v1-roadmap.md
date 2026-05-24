# V1 Roadmap

Archived after completion. This file records the work used to prepare the
stable `v1.0.0` release.

This roadmap tracks the work needed before `rudolint` should publish a stable
`v1.0.0` release. The v1 goal is contract stability: users should be able to
depend on the CLI, JSON output, configuration file, GitHub Action, release
artifacts, and documented rules without surprise.

Distribution channels such as Homebrew, crates.io publication, or package
manager integrations are intentionally out of scope for `v1.0.0`. They can be
added later in `v1.x` releases.

## 1. JSON Output Schema

Goal: make `rudolint check --format json` a documented, versioned contract for
CI systems and automation.

Subtasks:

1. Add `schemas/rudolint-findings-v1.schema.json`.
2. Document that `rudolint check --format json` emits the v1 findings schema.
3. Add a top-level `schemaVersion` field if the current JSON output does not
   already expose a schema version.
4. Validate existing JSON CLI snapshots against the schema in tests.
5. Add a test proving internal-only fields are not emitted accidentally.
6. Document the schema in `docs/output.md`.
7. Define the compatibility policy: breaking JSON changes require a new schema
   version.

Acceptance criteria:

- JSON output validates against a committed schema.
- The schema is referenced from user-facing docs.
- Breaking JSON changes are treated as explicit schema-version changes.

## 2. Config Schema

Goal: make `.rudolint.yaml` easy to validate in editors and CI before running
the linter.

Subtasks:

1. Add `schemas/rudolint-config-v1.schema.json`.
2. Cover all current config keys:
   - `ignore`
   - `select`
   - `severity`
   - `per-file-ignores`
   - `trusted-registries`
   - BuildKit entitlement opt-in settings
3. Add schema examples for common configs.
4. Validate fixture config files against the schema in tests.
5. Add README or docs links explaining how editors can use the schema.
6. Document config precedence:
   - CLI flags
   - explicit `--config`
   - discovered `.rudolint.yaml`
   - defaults
7. Define the compatibility policy: breaking config changes require a new schema
   version.

Acceptance criteria:

- Every documented config key is represented in the schema.
- Repo fixture configs validate against the schema.
- Config precedence is documented in one canonical place.

## 3. Real-World Regression Corpus

Goal: protect parser and rule behavior against realistic Dockerfile patterns,
not only focused rule fixtures.

Subtasks:

1. Create `fixtures/corpus/real-world/`.
2. Add curated Dockerfile categories:
   - multi-stage application images
   - Alpine package installs
   - Debian or Ubuntu package installs
   - BuildKit cache mounts
   - BuildKit secret mounts
   - BuildKit SSH mounts
   - multi-platform builds
   - heredocs
   - generated labels and metadata
3. Add a small metadata file per fixture explaining why the fixture exists.
4. Add parser snapshot tests for the corpus.
5. Add lint snapshot tests for the corpus.
6. Add negative/noise fixtures for Dockerfiles that should not trigger findings.
7. Document corpus update guidelines.
8. Ensure corpus tests are deterministic and require no network or Docker
   daemon.

Acceptance criteria:

- The corpus catches broad parser and lint regressions.
- False-positive-sensitive examples are represented.
- Corpus maintenance rules are documented.

## 4. Rule Coverage Matrix

Goal: make the supported v1 rule surface auditable at a glance.

Subtasks:

1. Reshape `docs/rule-roadmap.md` into a rule coverage matrix.
2. Add columns for:
   - rule ID
   - family
   - profile
   - default severity
   - documentation status
   - positive fixture status
   - negative fixture status
   - autofix status
   - source span coverage
3. Use these autofix statuses:
   - `safe`
   - `manual`
   - `not-applicable`
   - `not-yet`
4. Add a lightweight test or script that checks every implemented rule has a
   docs page.
5. Add a lightweight test or script that checks every implemented rule appears
   in the matrix.
6. Add a lightweight test or script that checks matrix entries do not mention
   unknown rules.
7. Document criteria for adding a new rule.
8. Keep planned future shell rules separate from the implemented v1 surface.

Acceptance criteria:

- Every implemented rule is visible in the matrix.
- Every implemented rule has documentation.
- Planned rules are not confused with the v1 supported surface.

## 5. Release Automation Polish

Goal: make future releases reproducible without manual release-body edits or
tribal knowledge.

Subtasks:

1. Update the release workflow so GitHub-generated "What's Changed" notes are
   automatically included.
2. Preserve cargo-dist install, download, checksum, and attestation sections
   below the generated notes.
3. Add a release workflow dry-run guard or validation step if feasible.
4. Update `docs/release.md` with the exact v1 release process.
5. Add a post-release verification checklist:
   - release exists
   - assets uploaded
   - checksums uploaded
   - installer works
   - Marketplace listing points to the release
   - GitHub Action example works
6. Document how to recover from a bad tag or failed release.
7. Ensure release notes automation works for `v1.0.0` and later patch releases.

Acceptance criteria:

- A tagged release produces complete release notes automatically.
- The release process is documented end to end.
- Bad tag or failed release recovery is documented.

## 6. V1 Action Tag Policy And Automation

Goal: support stable Marketplace and workflow usage after `v1.0.0` without
weakening reproducibility guidance.

Subtasks:

1. Document that stable users can pin `kubeply/rudolint@v1` after `v1.0.0`.
2. Define exact behavior: `v1` points to the latest stable `v1.x.y` release and
   never to prereleases.
3. Add a workflow or script to move the `v1` tag after a successful stable
   release.
4. Add guardrails so `v1` is not moved for prerelease tags.
5. After `v1.0.0`, update README examples to show both:
   - exact tag pinning for reproducibility
   - `v1` for stable automatic updates
6. Update `docs/action.md` with the same policy.
7. Add a release checklist item verifying the Marketplace listing references
   the intended tag pattern.
8. Add a test or script that checks README and action docs do not mention `v1`
   before the `v1` tag exists.

Acceptance criteria:

- `v1` tag movement is repeatable and guarded.
- Users can choose between exact pins and stable major-version pins.
- Marketplace guidance matches repository docs.
