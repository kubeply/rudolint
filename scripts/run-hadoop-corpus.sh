#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HADOOP_DOCKER_DIR="${HADOOP_DOCKER_DIR:-$ROOT/external-repos/hadoop/dev-support/docker}"
PROFILE="${RUDOLINT_HADOOP_PROFILE:-hadolint-compat}"

if [[ ! -d "$HADOOP_DOCKER_DIR" ]]; then
  echo "error: Hadoop Dockerfile directory not found: $HADOOP_DOCKER_DIR" >&2
  echo "clone Apache Hadoop into external-repos/hadoop or set HADOOP_DOCKER_DIR" >&2
  exit 2
fi

cd "$ROOT"

if [[ -n "${RUDOLINT_BIN:-}" ]]; then
  "$RUDOLINT_BIN" check "$HADOOP_DOCKER_DIR" \
    --no-config \
    --profile "$PROFILE" \
    --format text \
    --color never \
    --exit-zero
else
  cargo run -q -p rudolint -- check "$HADOOP_DOCKER_DIR" \
    --no-config \
    --profile "$PROFILE" \
    --format text \
    --color never \
    --exit-zero
fi

