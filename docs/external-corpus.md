# External Corpus

Rudolint uses external open-source repositories as advisory regression corpora.
These repositories are not vendored into this repo; workflows clone them on
demand so we can keep checking real Dockerfiles without carrying third-party
source trees.

## Apache Hadoop

Hadoop is useful because it has a compact but varied Dockerfile corpus under
`dev-support/docker`:

- Linux Dockerfiles named with underscore suffixes such as
  `Dockerfile_ubuntu_24`.
- A Windows Dockerfile using `SHELL ["cmd", "/S", "/C"]`, PowerShell commands,
  and Windows paths.
- Performance-oriented `DL3059` findings that are valid but intentionally
  advisory.
- Windows `DL3020` and `DL3045` cases that help audit copy/add path behavior.

Run it locally after cloning Hadoop into `external-repos/hadoop`:

```console
scripts/run-hadoop-corpus.sh
```

The script runs with `--exit-zero` because findings are a signal for product
analysis, not a failure condition for the external project. Parse failures,
panics, or CLI errors still fail the script.

The same check is available from the manual `external corpus` GitHub workflow.
