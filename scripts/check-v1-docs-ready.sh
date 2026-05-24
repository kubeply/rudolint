#!/usr/bin/env bash
set -euo pipefail

remote="${1:-origin}"
pattern='kubeply/rudolint@v1'
docs=(README.md docs/action.md)

if git ls-remote --exit-code --tags "$remote" refs/tags/v1 >/dev/null 2>&1; then
  exit 0
fi

matches="$(rg --fixed-strings --line-number "$pattern" "${docs[@]}" || true)"

if [[ -n "$matches" ]]; then
  echo "The v1 action tag does not exist on $remote, but docs advertise $pattern:" >&2
  echo "$matches" >&2
  echo "Create refs/tags/v1 after v1.0.0, or remove the v1 examples before release." >&2
  exit 1
fi
