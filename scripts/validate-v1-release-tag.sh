#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <release-tag>" >&2
  exit 2
fi

release_tag="$1"

if [[ ! "$release_tag" =~ ^v1\.[0-9]+\.[0-9]+$ ]]; then
  echo "refusing non-stable v1 release tag: $release_tag" >&2
  exit 1
fi
