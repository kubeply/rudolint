#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <generated-notes-json> <cargo-dist-body> <output-notes>" >&2
  exit 2
fi

generated_notes_json="$1"
cargo_dist_body="$2"
output_notes="$3"

jq -r '.body // ""' "$generated_notes_json" > "$output_notes"

if [[ -s "$cargo_dist_body" ]]; then
  {
    echo
    echo "## Install, Downloads, Checksums, And Attestations"
    echo
    sed -E \
      's#https://github\.com/kubeply/rudolint/releases/download/([^/[:space:]]+)/rudolint-installer\.sh#https://kubeply.com/rudolint/\1/install.sh#g' \
      "$cargo_dist_body"
  } >> "$output_notes"
fi
