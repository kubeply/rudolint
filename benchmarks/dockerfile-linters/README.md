# Dockerfile Linter Benchmarks

This benchmark suite compares `rudolint` with the main Dockerfile linting
alternatives using external CLI timing. It is separate from CodSpeed: CodSpeed
guards `rudolint` internals against regressions, while this suite answers the
user-facing question, "How fast is rudolint compared with other linters?"

## Compared Tools

- `rudolint`: this repository, built in release mode.
- `hadolint`: latest GitHub release for the host platform
  ([hadolint/hadolint](https://github.com/hadolint/hadolint)).
- `tally`: latest `tally-cli` package from npm
  ([npm](https://www.npmjs.com/package/tally-cli),
  [GitHub](https://github.com/wharflab/tally)).
- `docker build --check`: Docker's native BuildKit build checks through the
  locally installed Docker CLI
  ([Docker build checks](https://docs.docker.com/reference/build-checks/)).
- `dockerfilelint`: latest npm package
  ([npm](https://www.npmjs.com/package/dockerfilelint)).
- `dockerfile_lint`: latest npm package
  ([npm](https://www.npmjs.com/package/dockerfile_lint)).

Docker image scanners such as Trivy, Dockle, and Grype are intentionally not
included because they analyze built images or SBOMs rather than Dockerfile
source.

## Headline Result

The root README uses the 1,000-file repository benchmark as the headline chart:

![Dockerfile linter benchmark](results/headline.svg)

## Scenario Results

The secondary chart keeps the internal comparisons visible without crowding the
root README:

![Dockerfile linter benchmark scenarios](results/scenarios.svg)

## Methodology

The runner uses `hyperfine` for timing, with warmups and repeated runs. It
generates deterministic corpora under `target/dockerfile-linter-bench/corpus`
so benchmark inputs are reproducible but do not add 1,000 fixture files to the
repository.

Scenarios:

- single small Dockerfile
- single BuildKit-heavy Dockerfile
- 100 generated Dockerfiles
- 1,000 generated Dockerfiles
- JSON output for 100 generated Dockerfiles, where supported
- SARIF output for 100 generated Dockerfiles, where supported

The 100-file and 1,000-file corpora contain unique Dockerfiles. They vary base
images, package-manager commands, labels, ports, cache mounts, build arguments,
and copied paths. The benchmark does not duplicate the same Dockerfile hundreds
of times.

Rule coverage and diagnostic counts are not scored in these charts because each
tool has a different rule catalog. The suite measures CLI elapsed time for the
closest supported operation in each tool.

Lint findings are treated as successful benchmark executions. Exit codes are
normalized because Dockerfile linters disagree on contracts: some return `1`,
some return parser-specific values, and `dockerfile_lint` can return the number
of findings for a large batch.

## Reproduce

Install `hyperfine` first:

```bash
brew install hyperfine
```

Then run:

```bash
python3 scripts/dockerfile-linter-bench.py run --runs 5 --warmup 2
```

The script will:

1. build `rudolint` in release mode,
2. download the latest `hadolint` release for the host platform,
3. install the latest npm packages for `tally-cli`, `dockerfilelint`, and
   `dockerfile_lint` into `target/dockerfile-linter-bench/tools/node`,
4. use the local Docker CLI for `docker build --check`,
5. generate `results/latest.json`, `results/tool-versions.json`,
   `results/headline.svg`, and `results/scenarios.svg`.

Run a narrower pass while iterating:

```bash
python3 scripts/dockerfile-linter-bench.py run --scenario repo-1000 --runs 3 --warmup 1
```

## Current Tool Versions

The latest recorded benchmark run writes exact tool versions to
[`results/tool-versions.json`](results/tool-versions.json). Refresh that file
whenever the charts are regenerated.
