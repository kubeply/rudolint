# Implementation Plan

This plan orders the work needed to make `rudolint` usable as a fast
Dockerfile linter for local development, CI, GitHub Actions, and editor
integration.

The order is intentional:

1. Build the test environment first.
2. Stabilize CLI, diagnostics, and output contracts.
3. Harden parser and semantic models.
4. Add rules in small, verifiable batches.
5. Add packaging and release automation only after the binary contract is
   stable.

Every implementation task should be small enough to land independently with
focused fixtures and tests.

## Locked Decisions

These decisions are fixed unless a later implementation problem proves they
need to change.

1. Use `insta` snapshots heavily. Snapshots are the primary way to test real
   parser output, diagnostics, JSON, SARIF, CLI output, and fix previews.
2. Create a dedicated `rudolint-test` helper crate early. It should contain
   only centralized test helpers that are shared by multiple crates. Keep
   `Cargo.toml`, `.known-crates`, `crates/README.md`, and
   `docs/architecture.md` aligned when workspace crate inventory changes.
3. The JSON schema may break before `1.0.0`. Every schema change still needs
   snapshot updates so the change is visible in review.
4. Lock exit codes early:
   - `0`: no findings at or above threshold.
   - `1`: findings at or above threshold.
   - `2`: usage, config, or input error.
   - `3`: unexpected internal error.
5. Use `.rudolint.yaml` as the canonical discovered config file. Explicit
   `--config` always wins. Do not discover non-dot-prefixed `rudolint.yaml`.
6. Be explicit about rule provenance. `RDL` compatibility rules come from
   Hadolint behavior and should be documented that way. Project-native rules
   remain under `RDK` for BuildKit behavior and `RSC` for shell behavior.
7. The GitHub Action downloads released binaries only. It must not compile Rust
   in user workflows.
8. Release targets should be broad from day one, with Linux and macOS for
   `x86_64` and `aarch64`, plus portable Linux targets where release tooling
   supports them cleanly.
9. Autofix is part of rule completion. A rule is not complete until it has
   either a tested safe fix or explicit metadata and tests proving that no
   safe automatic fix can be inferred.
10. Compatibility oracle tests should track the latest supported version of the
    compared tool, such as Hadolint, through an explicit update workflow. The
    runtime binary still never shells out to those tools.
11. Keep `action.yml` in this repository and publish it with the first binary
    release so the GitHub Action contract stays aligned with `rudolint`
    releases.
12. `rudolint-shell` owns a native, lightweight shell analysis layer. It starts
    with tokenization and source spans, then grows into command facts for
    Dockerfile `RUN` analysis. It does not try to execute or fully model Bash.
    External shell tools may be used only as optional test oracles, never at
    runtime.
13. Until `1.0.0`, the pinned `rust-toolchain.toml` is the exact project
    toolchain, not a minimum supported Rust version.

## Milestone 1: Test Harness And Fixtures

Goal: make every future parser, rule, CLI, and output change easy to verify
before broad implementation starts.

1. Create a fixture layout under `fixtures/`:
   - `fixtures/parser/`
   - `fixtures/rules/`
   - `fixtures/cli/`
   - `fixtures/buildkit/`
   - `fixtures/compat/`
   - `fixtures/corpus/`
2. Define fixture directory naming:
   - parser fixtures: `parser/<feature-name>/Dockerfile`
   - rule fixtures: `rules/<rule-id>.<short-name>/Dockerfile`
   - CLI fixtures: `cli/<behavior-name>/`
   - compatibility fixtures: `compat/<rule-id>.<short-name>/Dockerfile`
3. Define per-fixture files:
   - `Dockerfile`: input file.
   - `.rudolint.yaml`: optional fixture-specific config.
   - `expected.json`: normalized diagnostic expectation when snapshots are not
     enough.
   - `README.md`: only for unusual fixtures that need context.
4. Add a `crates/rudolint-test` helper crate.
5. Keep `rudolint-test` dev-only. Production crates must not depend on it.
6. Add helper functions for:
   - resolving fixture paths.
   - reading fixture Dockerfiles.
   - invoking the CLI binary with `assert_cmd`.
   - normalizing JSON output.
   - normalizing SARIF output.
   - replacing absolute paths with stable placeholders.
   - rendering fix previews.
   - asserting snapshot names are stable.
7. Add `insta` snapshots for JSON diagnostics.
8. Add `insta` snapshots for SARIF diagnostics.
9. Add `insta` snapshots for parser facts.
10. Add `insta` snapshots for fix previews.
11. Add a small number of human-output smoke snapshots.
12. Add parser fixture tests for:
   - simple instructions.
   - comments.
   - parser directives.
   - line continuations.
   - heredocs.
   - `RUN --mount`.
   - `FROM --platform`.
   - `COPY --from`.
13. Add rule fixture tests that parse a Dockerfile and call `RuleEngine`
    directly.
14. Add CLI integration tests for:
    - explicit Dockerfile path.
    - directory discovery.
    - stdin.
    - stdin with future `--stdin-filename`.
    - config file loading.
    - config ignore.
    - config severity override.
    - JSON output.
    - SARIF output.
    - exit status with findings.
    - exit status without findings.
15. Add ignored compatibility oracle tests gated by environment variables:
    - `RUDOLINT_ORACLE_BIN`
    - `RUDOLINT_UPDATE_ORACLE=1`
16. Add an oracle update workflow that records:
    - compared tool name.
    - compared tool version.
    - download source or installation method.
    - normalized output snapshot.
17. Add a maintenance command for oracle refreshes, with this intended shape:
    `cargo run -p xtask -- update-oracle hadolint`.
18. Compare against the latest supported oracle version when refreshing
    compatibility fixtures. Keep the version visible in snapshot metadata so
    rule drift is reviewable.
19. Define oracle normalization:
    - input: external linter output.
    - output: stable JSON with path, line, column, code, severity, and message.
    - all absolute paths must be stripped.
20. Add a tiny benchmark corpus:
    - one small Dockerfile.
    - one medium multi-stage Dockerfile.
    - one generated large Dockerfile.
    - one generated directory tree.
21. Add a benchmark helper in `rudolint-bench` for loading corpus metadata.

Acceptance criteria:

- `cargo test --all-targets --all-features --locked` runs fixture tests.
- Snapshot output is stable across machines.
- Shared fixture, snapshot, CLI, and normalization helpers live in
  `rudolint-test`.
- Ignored oracle tests can be run locally without changing default CI.
- Oracle snapshots show the compared tool version so updates are intentional
  and reviewable.
- The fixture format is documented enough that every rule can use it.

## Milestone 2: CI Quality Gates

Goal: keep every pull request trustworthy while the implementation grows.

1. Keep the existing quality job as the primary blocking job.
2. Ensure CI runs:
   - `cargo fmt --all -- --check`
   - `cargo check --all-targets --all-features --locked`
   - `cargo clippy --all-targets --all-features --locked -- -D warnings`
   - `cargo test --all-targets --all-features --locked`
   - `cargo deny check`
   - `actionlint`
3. Add `cargo test --doc --locked` once crate docs contain examples.
4. Add a snapshot consistency check with:
   `INSTA_UPDATE=no cargo test --all-targets --all-features --locked`.
5. Add `cargo-insta` to the documented local toolchain once reviewing and
   accepting snapshots becomes common.
6. Add a separate benchmark smoke job that builds benchmark code but does not
   enforce performance numbers yet.
7. Add a separate optional oracle job, disabled until the oracle update workflow
   can install the latest supported compared tool reproducibly.
8. Add CI caching checks for Rust builds through the existing cache action.
9. Keep Depot runners as the default runner target.
10. Add a CI comment in workflow files explaining that GitHub Action
   tests must use released binaries instead of compiling Rust in user jobs.

Acceptance criteria:

- Default CI fails on formatting, clippy warnings, broken tests, denied
  dependencies, broken snapshots, and invalid workflow syntax.
- Oracle and benchmark jobs do not slow down normal pull requests until they
  become reliable enough to enforce.

## Milestone 3: CLI Contract

Goal: make the binary useful in local workflows and lightweight CI jobs.

1. Add explicit CLI exit code documentation matching the locked contract.
2. Implement stable exit code mapping in `rudolint-cli`.
3. Add `rudolint --version --json`.
4. Add `rudolint check --exit-zero`.
5. Add `rudolint check --no-config`.
6. Add `rudolint check --stdin-filename <path>`.
7. Add `rudolint check --quiet`.
8. Add `rudolint check --verbose`.
9. Add `rudolint check --show-source`.
10. Add `rudolint rules --format json`.
11. Add `rudolint explain <RULE_ID>`.
12. Add `rudolint config --show` later, after settings resolution is stable.
13. Add CLI tests for every option above.

Acceptance criteria:

- CI users can run the binary without project-specific setup.
- Stdin use is viable for editors and pre-commit hooks.
- JSON and SARIF output do not depend on terminal state.

## Milestone 4: Diagnostics And Source Model

Goal: make diagnostics precise enough for SARIF, editors, autofix, and future
GitHub annotations.

1. Move source text ownership into `rudolint-source`.
2. Add a line index type that maps byte offsets to line and column.
3. Add `SourceFile` with:
   - path.
   - display path.
   - source text.
   - line index.
4. Add `Span` with start byte, end byte, start line, start column, end line, and
   end column.
5. Update `Finding` to carry:
   - rule code.
   - severity.
   - message.
   - path.
   - primary span.
   - optional labels.
   - optional help text.
6. Update output renderers to use spans instead of instruction-only line
   numbers.
7. Add tests for UTF-8 column handling.
8. Add tests for CRLF normalization.
9. Add tests for trailing newline and no-trailing-newline inputs.

Acceptance criteria:

- Every diagnostic has stable path, line, column, and rule code.
- SARIF locations point to useful ranges.
- Future autofix can reuse the same span model.

## Milestone 5: Dockerfile Parser Hardening

Goal: make parsing correct enough that rules do not need ad hoc string
inspection.

1. Parse parser directives:
   - `# syntax=`
   - `# escape=`
   - `# check=`
2. Preserve comments with source spans.
3. Parse instruction keyword and raw argument span.
4. Parse escaped line continuations.
5. Support both Unix and Windows escape modes.
6. Parse JSON-form instructions as JSON when valid.
7. Preserve invalid JSON-form instructions as recoverable parse facts.
8. Parse shell-form instructions as raw shell text with spans.
9. Parse heredocs:
   - delimiter.
   - quoted delimiter.
   - target instruction.
   - body span.
10. Parse `FROM`:
    - flags.
    - image reference.
    - digest.
    - `AS` alias.
    - `--platform`.
11. Parse `RUN`:
    - flags.
    - `--mount`.
    - `--network`.
    - `--security`.
    - shell body.
12. Parse `COPY` and `ADD`:
    - flags.
    - `--from`.
    - `--chown`.
    - `--chmod`.
    - sources.
    - destination.
13. Parse `HEALTHCHECK` flags.
14. Parse `ARG` names and default values.
15. Parse `ENV` names and values for both `key=value` and legacy pair forms.
16. Parse `EXPOSE` ports and protocols.
17. Add parser recovery for unknown or malformed instructions.
18. Add parser snapshots for each feature.
19. Cite upstream Dockerfile frontend or BuildKit references in fixtures or rule
    docs when behavior depends on BuildKit semantics.

Acceptance criteria:

- Rules consume typed parser facts for common Dockerfile constructs.
- Invalid Dockerfiles can still produce useful diagnostics where possible.
- BuildKit features are parsed structurally.

## Milestone 6: Semantic Model

Goal: provide rule-friendly facts without putting policy inside the parser.

1. Add stage model:
   - stage index.
   - stage alias.
   - base image.
   - stage platform.
   - instruction range.
2. Add image model in `rudolint-image`:
   - registry.
   - repository.
   - tag.
   - digest.
   - local stage reference.
3. Add ARG scope model:
   - global args before the first `FROM`.
   - stage args.
   - default values.
   - inherited values.
4. Add ENV model:
   - stage-local env vars.
   - final effective env vars.
5. Add copy graph:
   - `COPY` sources.
   - `ADD` sources.
   - destination paths.
   - `--from` stage or image.
6. Add BuildKit feature model in `rudolint-buildkit`:
   - frontend syntax reference.
   - frontend version.
   - mounts.
   - heredoc usage.
   - network mode.
   - security mode.
7. Add package manager detection:
   - `apt-get`
   - `apt`
   - `apk`
   - `dnf`
   - `yum`
   - `microdnf`
   - `pip`
   - `npm`
   - `pnpm`
   - `yarn`
   - `cargo`
   - `go`
8. Add multi-platform facts:
   - `TARGETPLATFORM`.
   - `BUILDPLATFORM`.
   - `TARGETARCH`.
   - `TARGETOS`.
   - stage platform pins.
9. Add semantic model tests independent from rule tests.

Acceptance criteria:

- Rule implementations can be short and declarative.
- BuildKit rules do not grep raw Dockerfile text.
- Compatibility rules and BuildKit-native rules use the same underlying facts.

## Milestone 7: Config And Policy

Goal: make rule selection predictable for real repositories.

1. Stabilize `.rudolint.yaml` schema.
2. Add config fields:
   - `select`.
   - `ignore`.
   - `extend-ignore`.
   - `severity`.
   - `trusted-registries`.
   - `allow-entitlements`.
   - `per-file-ignores`.
3. Add config discovery order:
   - explicit `--config`.
   - nearest `.rudolint.yaml`.
   - no config when `--no-config` is set.
4. Add config validation errors with line and column when possible.
5. Add settings resolver in `rudolint-settings`.
6. Move effective settings out of `rudolint-cli`.
7. Add profile behavior in `rudolint-policy`:
   - `default`.
   - `compat`.
   - future `strict`.
8. Add inline suppression parsing for project-native comments.
9. Keep compatibility warnings for legacy external suppression comments.
10. Add tests for config precedence and rule selection.

Acceptance criteria:

- A CI run can be reproduced locally with the same config file.
- Users can incrementally adopt the tool by ignoring specific rules.
- Compatibility mode emits only intended compatibility diagnostics.

## Milestone 8: Rule Engine Refactor

Goal: make rule implementation scale without one large `core.rs` file.

1. Split `rudolint-rules/src/core.rs` into modules:
   - `catalog.rs`
   - `engine.rs`
   - `metadata.rs`
   - `compat/`
   - `buildkit/`
   - `shell/`
2. Add `RuleMetadata`:
   - code.
   - name.
   - summary.
   - default severity.
   - profile.
   - category.
   - status.
   - docs URL.
   - autofix availability.
3. Add a rule registration macro only for metadata boilerplate.
4. Keep rule behavior as ordinary Rust functions or structs.
5. Add a catalog test that checks:
   - no duplicate rule codes.
   - all codes have metadata.
   - all implemented codes appear in docs.
   - all docs codes appear in the catalog.
6. Add `rules --implemented` test.
7. Add `rules --format json` test.
8. Add docs synchronization checks for `docs/rules/`.

Acceptance criteria:

- Adding one rule touches one rule file, one fixture directory, and docs.
- Rule metadata is queryable by CLI, docs, and future LSP.
- Rule docs include provenance, examples, config notes, and fix behavior.

## Milestone 9: Autofix Foundation

Goal: make fixability part of the rule contract before large rule batches land.

1. Define edit primitives in `rudolint-fix`.
2. Add text replacement, insertion, and deletion types.
3. Add patch conflict detection.
4. Add a `FixApplicability` model:
   - safe automatic fix.
   - unsafe/manual fix.
   - no fix available.
5. Add rule metadata for fix availability.
6. Add `rudolint check --fix --dry-run`.
7. Add `rudolint check --fix`.
8. Add JSON output for proposed fixes.
9. Add `insta` snapshots for fix previews.
10. Add tests proving fixes are idempotent.
11. Add tests proving fixes do not overlap or corrupt source text.
12. Add safe fix for inserting `# syntax=` when BuildKit features are used.
13. Add safe fix for simple `MAINTAINER` replacement only if exact output is
    deterministic.
14. Add safe fix suggestions for JSON-form `CMD` and `ENTRYPOINT`, but do not
    auto-apply unless command tokenization is reliable.
15. Require every rule to declare one of:
    - tested safe fix.
    - tested manual fix suggestion.
    - tested no-fix rationale when a correct edit cannot be inferred.

Acceptance criteria:

- `--fix --dry-run` is trustworthy before `--fix` is encouraged.
- Rule completion includes fix behavior or an explicit no-fix rationale.
- No autofix rewrites shell semantics accidentally.

## Milestone 10: Compatibility Rule Batch 1

Goal: make the tool useful on common Dockerfiles before chasing edge cases.

1. Tighten existing `RDL1001` fixture coverage.
2. Tighten existing `RDL3000` fixture coverage for absolute `WORKDIR`.
3. Tighten existing `RDL3002` fixture coverage for final root user.
4. Tighten existing `RDL3006` fixture coverage for missing base image tags.
5. Tighten existing `RDL3007` fixture coverage for `latest` tags.
6. Tighten existing `RDL3011` fixture coverage for invalid `EXPOSE`.
7. Tighten existing `RDL3012` fixture coverage for duplicate `HEALTHCHECK`.
8. Tighten existing `RDL3020` fixture coverage for `ADD` vs `COPY`.
9. Tighten existing `RDL3024` fixture coverage for duplicate stage aliases.
10. Tighten existing `RDL3025` fixture coverage for shell-form entrypoints.
11. Tighten existing `RDL4000` fixture coverage for `MAINTAINER`.
12. Tighten existing `RDL4003` fixture coverage for duplicate `CMD`.
13. Tighten existing `RDL4004` fixture coverage for duplicate `ENTRYPOINT`.
14. Implement `RDL3001` with fixtures.
15. Implement `RDL3003` with fixtures.
16. Implement `RDL3004` with fixtures.
17. Implement `RDL3008` with fixtures.
18. Implement `RDL3009` with fixtures.
19. Implement `RDL3010` with fixtures.
20. Implement `RDL3013` with fixtures.
21. Implement `RDL3014` with fixtures.
22. Implement `RDL3015` with fixtures.
23. Implement `RDL3016` with fixtures.
24. Implement `RDL3018` with fixtures.
25. Implement `RDL3019` with fixtures.

Acceptance criteria:

- Common Dockerfile quality issues are caught.
- Every rule has at least one positive and one negative fixture.
- Every rule has a tested fix, manual suggestion, or no-fix rationale.
- Compatibility profile output is snapshot-tested.
- `RDL` rule docs identify Hadolint compatibility provenance.

## Milestone 11: Compatibility Rule Batch 2

Goal: cover copy, add, package-manager, and metadata rules after parser and
semantic facts are solid.

1. Implement `RDL3021` with fixtures.
2. Implement `RDL3022` with fixtures.
3. Implement `RDL3023` with fixtures.
4. Implement `RDL3026` with fixtures.
5. Implement `RDL3027` with fixtures.
6. Implement `RDL3028` with fixtures.
7. Implement `RDL3029` with fixtures.
8. Implement `RDL3030` with fixtures.
9. Implement `RDL3032` with fixtures.
10. Implement `RDL3033` with fixtures.
11. Implement `RDL3034` with fixtures.
12. Implement `RDL3035` with fixtures.
13. Implement `RDL3036` with fixtures.
14. Implement `RDL3037` with fixtures.
15. Implement `RDL3038` with fixtures.
16. Implement `RDL3040` with fixtures.
17. Implement `RDL3041` with fixtures.
18. Implement `RDL3042` with fixtures.
19. Implement `RDL3043` with fixtures.
20. Implement `RDL3044` with fixtures.
21. Implement `RDL3045` with fixtures.
22. Implement `RDL3046` with fixtures.
23. Implement `RDL3047` with fixtures.
24. Implement `RDL3048` with fixtures.
25. Implement `RDL3049` with fixtures.
26. Implement `RDL3050` with fixtures.
27. Implement `RDL3051` with fixtures.
28. Implement `RDL3052` with fixtures.
29. Implement `RDL3053` with fixtures.
30. Implement `RDL3054` with fixtures.
31. Implement `RDL3055` with fixtures.
32. Implement `RDL3056` with fixtures.
33. Implement `RDL3057` with fixtures.
34. Implement `RDL3058` with fixtures.
35. Implement `RDL3059` with fixtures.
36. Implement `RDL3060` with fixtures.
37. Implement `RDL3061` with fixtures.
38. Implement `RDL3062` with fixtures.
39. Implement `RDL3063` with fixtures.
40. Implement `RDL4001` with fixtures.
41. Implement `RDL4005` with fixtures.
42. Implement `RDL4006` with fixtures.

Acceptance criteria:

- Planned compatibility rule IDs are either implemented or explicitly marked
  blocked by parser or shell analysis gaps.
- No package-manager rule relies on fragile substring checks once shell token
  facts exist.
- Every rule has a tested fix, manual suggestion, or no-fix rationale.

## Milestone 12: BuildKit-Native Rule Batch

Goal: make `rudolint` meaningfully better for modern Docker builds instead of
only matching compatibility rules.

1. [x] Reimplement `RDK1000` on typed BuildKit feature facts.
2. [x] Reimplement `RDK1001` using ARG and ENV semantic facts.
3. [x] Reimplement `RDK1002` using shell and secret mount facts.
4. [x] Reimplement `RDK1003` using package-manager and cache mount facts.
5. [x] Implement `RDK1004`: secret mount target copied into an image layer.
6. [x] Implement `RDK1005`: SSH mount used without explicit command scoping.
7. [x] Implement `RDK1006`: cache mount missing stable `id` in multi-stage
   builds.
8. [x] Implement `RDK1007`: cache mount sharing mode unsafe for common package
   managers.
9. [x] Implement `RDK1008`: BuildKit network or security entitlement used
   without config opt-in.
10. [x] Implement `RDK1009`: multi-platform build uses host architecture
    accidentally.
11. [x] Implement `RDK1010`: frontend version too old for the used syntax.
12. [x] Add BuildKit fixtures for every mount type:
    - [x] `cache`.
    - [x] `secret`.
    - [x] `ssh`.
    - [x] `bind`.
    - [x] `tmpfs`.
13. [x] Add Buildx fixtures for:
    - [x] `TARGETPLATFORM`.
    - [x] `BUILDPLATFORM`.
    - [x] `TARGETARCH`.
    - [x] `TARGETOS`.
    - [x] `FROM --platform`.

Acceptance criteria:

- Default profile catches BuildKit-specific risks that compatibility linters
  cannot model well.
- Compatibility profile can suppress BuildKit-native diagnostics cleanly.
- Every BuildKit rule has a tested fix, manual suggestion, or no-fix rationale.

## Milestone 13: Shell Analysis

Goal: add useful `RUN` diagnostics without creating noisy substring-based
false positives.

1. [x] Add native shell tokenization in `rudolint-shell`.
2. [x] Track token spans back to Dockerfile source spans.
3. [x] Recognize single-quoted and double-quoted strings.
4. [x] Recognize escapes.
5. [x] Recognize variable expansions.
6. [x] Recognize environment assignments.
7. [x] Recognize simple commands.
8. [x] Recognize pipelines.
9. [x] Recognize command chains with `&&` and `||`.
10. [x] Recognize redirections.
11. [x] Recognize command substitutions.
12. [x] Build package-manager command facts on top of shell tokens.
13. [x] Add shell fixture snapshots.
14. [x] Implement `RSC2086` after quoting facts are reliable.
15. [x] Implement `RSC2046` after command substitution facts are reliable.
16. [x] Implement `RSC2015` after command-chain facts are reliable.
17. [x] Implement `RSC2164` after `cd` command facts are reliable.
18. [x] Implement `RSC2155` after assignment facts are reliable.
19. [x] Add the low-risk tracked `RSC` follow-up batch (`RSC2002`, `RSC2181`)
    after fixture coverage shows low false-positive risk.

Additional tracked `RSC` IDs remain in `docs/rule-roadmap.md` and should move
into future implementation milestones only when fixture coverage demonstrates
low false-positive risk.

Acceptance criteria:

- Shell rules are based on shell facts, not raw substring matching.
- Shell diagnostics point to the relevant part of the `RUN` instruction.
- Every shell rule has a tested fix, manual suggestion, or no-fix rationale.
- Runtime shell analysis uses `rudolint-shell`, not external shell tools.

## Milestone 14: Output Formats

Goal: support terminal users, CI systems, SARIF upload, and future editor
integrations with stable output.

1. [x] Document that JSON schema may break before `1.0.0`.
2. [x] Document JSON schema in `docs/output.md`.
3. [x] Snapshot JSON output for multiple files.
4. [x] Snapshot JSON output for stdin.
5. [x] Snapshot JSON output with config overrides.
6. [x] Stabilize SARIF output.
7. [x] Validate SARIF output against GitHub code scanning expectations.
8. [x] Add SARIF snapshot for multiple files.
9. [x] Add SARIF snapshot for source spans.
10. [x] Add human renderer grouping by file.
11. [x] Add human renderer source excerpts behind `--show-source`.
12. [x] Defer a future GitHub annotation format unless SARIF proves
    insufficient for a specific CI workflow.

Acceptance criteria:

- JSON changes are visible in review through snapshots until `1.0.0`.
- SARIF can be uploaded in GitHub Actions without post-processing.
- Human output stays readable and compact.

## Milestone 15: Performance And Memory

Goal: keep the binary fast enough for pre-commit hooks and large CI jobs.

1. [x] Add cold-start benchmark.
2. [x] Add one-file lint benchmark.
3. [x] Add recursive directory benchmark.
4. [x] Add JSON rendering benchmark.
5. [x] Add SARIF rendering benchmark.
6. [x] Add large generated Dockerfile benchmark.
7. [x] Add memory measurement notes for local benchmark runs.
8. [x] Maintain performance budget documentation in `docs/performance.md`.
9. [x] Start with advisory targets:
   - cold start under 50 ms on a typical CI Linux runner.
   - one Dockerfile under 20 ms after process start.
   - 1,000 small Dockerfiles under 2 seconds.
   - JSON overhead under 15 percent on large fixture sets.
   - SARIF overhead under 30 percent on large fixture sets.
10. [x] Avoid blocking CI on performance numbers until benchmarks are stable.
11. [x] Add an advisory benchmark job on main branch only.
12. [x] Split CodSpeed benchmarks into path-scoped jobs for parser, lint,
    end-to-end, batch, output, and CLI workloads.

Acceptance criteria:

- Performance regressions are visible before releases.
- Benchmark fixtures are reproducible from the repo.

## Milestone 16: LSP And Editor Integration

Goal: reuse the same parser, settings, diagnostics, and output model for
editors without changing CLI behavior.

1. [x] Add LSP diagnostic conversion in `rudolint-lsp`.
2. [x] Add document-open linting.
3. [x] Add document-change linting.
4. [x] Add config discovery for editor workspaces.
5. [x] Add rule explanation hover support.
6. [x] Add code actions only for safe fixes from `rudolint-fix`.
7. [x] Add LSP tests for diagnostic ranges.
8. [x] Add a stdio language server binary for terminal and editor clients.
9. [x] Add end-to-end stdio LSP protocol tests.
10. [x] Document release packaging for `rudolint-lsp`.

Acceptance criteria:

- LSP consumes the same engine as the CLI.
- Editor diagnostics match CLI diagnostics for the same file and config.

## Milestone 17: Release Packaging

Goal: make installation fast and lightweight for CI without requiring Rust on
user machines.

1. [x] Choose release tooling, likely `cargo-dist` unless it conflicts with the
   repository constraints.
2. [x] Keep `rust-toolchain.toml` as the exact release toolchain until `1.0.0`.
3. [x] Produce release binaries for:
   - [x] `x86_64-unknown-linux-gnu`.
   - [x] `aarch64-unknown-linux-gnu`.
   - [x] `x86_64-unknown-linux-musl`, if release tooling supports it cleanly.
   - [x] `aarch64-unknown-linux-musl`, if release tooling supports it cleanly.
   - [x] `x86_64-apple-darwin`.
   - [x] `aarch64-apple-darwin`.
4. [x] Defer Windows targets until path handling and shell behavior are tested
   on Windows CI.
5. [x] Generate checksums for all artifacts.
6. [x] Generate release notes from conventional commits or curated release notes.
7. [x] Add provenance or attestations if release tooling supports them cleanly.
8. [x] Add a release dry-run workflow.
9. [x] Add a tagged release workflow.
10. [x] Add install script that downloads pinned release artifacts.
11. [x] Add documentation for:
    - [x] direct binary download.
    - [x] install script.
    - [x] `cargo install`.
    - [x] Docker image non-goal until there is a concrete CI use case.
12. [x] Keep local-machine releases forbidden in `CONTRIBUTING.md`.

Acceptance criteria:

- CI users can install `rudolint` without compiling Rust.
- Release artifacts are checksummed.
- Release automation is reproducible from GitHub Actions.

## Milestone 18: GitHub Action

Goal: provide a first-class CI entrypoint while keeping the action thin and
fast.

1. [x] Add `action.yml` in this repository as part of the first binary release.
2. [x] Publish the action and release binary together so versions stay aligned.
3. [x] Make the action download a released binary by version.
4. [x] Do not compile Rust inside the user action path.
5. [x] Add action inputs:
   - [x] `version`.
   - [x] `paths`.
   - [x] `config`.
   - [x] `profile`.
   - [x] `format`.
   - [x] `failure-threshold`.
   - [x] `sarif-output`.
   - [x] `upload-sarif`.
6. [x] Add action outputs:
   - [x] `exit-code`.
   - [x] `findings-count`.
   - [x] `sarif-path`.
7. [x] Add examples for:
   - [x] simple check.
   - [x] monorepo path check.
   - [x] SARIF upload.
   - [x] compatibility profile.
8. [x] Add self-test workflow that uses the local action against fixtures.
9. [x] Add an action versioning policy.

Acceptance criteria:

- A GitHub user can add the action with a few YAML lines.
- The action is fast because it downloads a binary instead of compiling.
- SARIF upload works without custom glue.
- `action.yml` and release artifacts are versioned from the same repository.

## Milestone 19: First Usable Release Criteria

The first usable release should not require every planned rule. It should meet
these criteria instead:

1. [x] Parser handles common Dockerfiles, BuildKit mounts, heredocs, stage aliases,
   and platform flags.
2. [x] JSON and SARIF schemas are stable enough for CI use.
3. [x] CLI exit codes are documented and tested.
4. [x] Config supports ignore, severity overrides, trusted registries, and
   BuildKit entitlement opt-ins.
5. [x] At least the existing compatibility rules are fixture-backed and reliable.
6. [x] At least `RDK1000` through `RDK1003` are typed-fact based and reliable.
7. [x] Release binaries are available for common Linux and macOS targets.
8. [x] GitHub Action can run without Rust installed.
9. [x] CI gates formatting, clippy, tests, dependency policy, snapshots, and
   workflow syntax.
10. [x] Known limitations are documented clearly.

## Milestone 20: GitHub Marketplace Publication

Goal: make the GitHub Action discoverable without changing the release or
runtime contract.

1. [ ] Accept the GitHub Marketplace Developer Agreement for the publishing
   account or organization.
2. [x] Confirm `action.yml` has Marketplace-ready metadata:
   - [x] name.
   - [x] description.
   - [x] branding.
   - [x] complete inputs and outputs.
3. [x] Confirm README and `docs/action.md` include copy-paste examples for:
   - [x] basic check.
   - [x] monorepo path check.
   - [x] SARIF upload.
   - [x] compatibility profile.
4. [x] Document the manual Marketplace publication checklist.
5. [ ] Publish the released action to GitHub Marketplace from the existing release
   tag.
6. [ ] Verify the Marketplace listing points users to the pinned release workflow
   pattern.
7. [ ] Add the Marketplace link to README after publication.

Acceptance criteria:

- The action is discoverable on GitHub Marketplace.
- Marketplace users can install it with a pinned release tag.
- The Marketplace listing does not imply floating major tags are available
  before the project commits to that compatibility policy.

## Execution Notes

- Prefer one rule per pull request once the fixture harness exists.
- Do not add new rule behavior without a positive fixture, negative fixture,
  snapshot, and fix behavior declaration.
- Do not add shell diagnostics by substring matching.
- Do not let output renderers influence rule behavior.
- Do not make the GitHub Action compile Rust in user workflows.
- Keep Hadolint-derived compatibility behavior in `RDL`.
- Keep shell behavior in `RSC`.
- Keep BuildKit-native behavior in `RDK`.
- Keep runtime linting independent from external oracle tools.
