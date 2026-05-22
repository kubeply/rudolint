# Performance Budgets

These budgets are advisory until benchmark fixtures are stable. They exist to
keep design decisions honest before the first usable release.

## Benchmark Tracks

`rudolint` uses two performance tracks:

- CodSpeed PR benchmarks for parser, lint, end-to-end, batch, output
  rendering, and CLI-style workloads. The workflow splits these into
  independent jobs so path-scoped changes run only the relevant benchmark group.
- A main-branch-only advisory workflow for process cold-start timing, recursive
  directory timing, and memory measurements from the release binary.

The checked-in corpus under `fixtures/corpus/` is the source of truth for both
tracks:

- `small`: one minimal Dockerfile.
- `medium-multistage`: one representative multi-stage Dockerfile.
- `large-generated`: one large generated Dockerfile.
- `buildkit-heavy`: one Dockerfile with BuildKit-heavy syntax.
- `directory-tree`: a small recursive tree for directory-discovery benchmarks.

## CodSpeed Groups

The CodSpeed workflow is intentionally split by benchmark target:

- `parser`: Dockerfile parsing over the corpus files.
- `lint`: one-file linting against pre-parsed Dockerfiles.
- `end_to_end`: parse-and-lint over the corpus files.
- `batch`: the 1,000-file parse-and-lint workload.
- `output`: JSON and SARIF rendering over a large diagnostic set.
- `cli`: release-binary cold start plus one-file and recursive-directory CLI
  checks.

On pull requests, a lightweight selector job maps changed paths to the smallest
benchmark groups that still cover the affected code. Pushes to `main` and
manual dispatches run every group.

## Advisory Workflow

The `Performance advisory` workflow runs only on `main` and manual dispatch. It
builds the release binary, records toolchain and OS details, then runs:

- `hyperfine` cold-start timing for `rudolint --version`.
- `hyperfine` one-file lint timing.
- `hyperfine` recursive directory lint timing.
- `/usr/bin/time -v` memory measurements for JSON and SARIF rendering on the
  large generated corpus.

The workflow uploads timing JSON artifacts, but it does not block merges on
performance numbers yet.

## Local Memory Notes

Use the release binary when measuring memory locally:

```bash
cargo build -p rudolint --release --locked
/usr/bin/time -v target/release/rudolint check fixtures/corpus/large-generated/Dockerfile --format json --failure-threshold ignore >/tmp/rudolint-json.out
/usr/bin/time -v target/release/rudolint check fixtures/corpus/large-generated/Dockerfile --format sarif --failure-threshold ignore >/tmp/rudolint-sarif.out
```

On macOS, use `/usr/bin/time -l` instead of `/usr/bin/time -v`. Record the
operating system, CPU, target triple, commit SHA, and exact command line with
the result.

## Advisory Targets

Initial targets:

- cold start: under 50 ms on a typical CI Linux runner.
- one Dockerfile: under 20 ms after process start.
- 1,000 small Dockerfiles: under 2 seconds.
- JSON output overhead: under 15 percent of lint time for large fixture sets.
- SARIF output overhead: under 30 percent of lint time for large fixture sets.
- release binary size: track every release; warn on unexpected growth over 20
  percent.

Benchmark runs should record hardware, OS, target triple, commit SHA, and
command line. CI performance jobs should start advisory-only and run on `main`
until the corpus is stable enough to gate pull requests.
