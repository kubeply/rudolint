#!/usr/bin/env bash
set -euo pipefail

remote="${1:-origin}"
pattern='kubeply/rudolint@v1'
docs=(README.md docs/action.md)

if ! git ls-remote --exit-code --tags "$remote" refs/tags/v1 >/dev/null 2>&1; then
  echo "refs/tags/v1 does not exist on $remote; skipping v1 docs availability check." >&2
  exit 0
fi

missing=0
for doc in "${docs[@]}"; do
  if ! rg --fixed-strings --quiet "$pattern" "$doc"; then
    echo "$doc should document $pattern once refs/tags/v1 exists." >&2
    missing=1
  fi
done

exit "$missing"
