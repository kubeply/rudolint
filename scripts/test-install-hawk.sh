#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/install-hawk-path.sh
source "${script_directory}/install-hawk-path.sh"

destination="/opt/rudolint-test-cargo/bin"
expected_warning="warning: add ${destination} to PATH before running 'cargo hawk'"

present_output="$(
  PATH="/usr/bin:${destination}:/bin" warn_if_hawk_bin_not_on_path "${destination}" 2>&1
)"
if [[ -n "${present_output}" ]]; then
  echo "expected no warning when ${destination} is on PATH" >&2
  exit 1
fi

missing_output="$(
  PATH="/usr/bin:/bin" warn_if_hawk_bin_not_on_path "${destination}" 2>&1
)"
if [[ "${missing_output}" != "${expected_warning}" ]]; then
  echo "expected a warning when ${destination} is missing from PATH" >&2
  exit 1
fi

metacharacter_destination="/opt/rudolint-test[01]*?/bin"
metacharacter_present_output="$(
  PATH="/usr/bin:${metacharacter_destination}:/bin" \
    warn_if_hawk_bin_not_on_path "${metacharacter_destination}" 2>&1
)"
if [[ -n "${metacharacter_present_output}" ]]; then
  echo "expected PATH metacharacters to be compared literally" >&2
  exit 1
fi

metacharacter_missing_output="$(
  PATH="/usr/bin:/opt/rudolint-test0-other/bin:/bin" \
    warn_if_hawk_bin_not_on_path "${metacharacter_destination}" 2>&1
)"
expected_metacharacter_warning="warning: add ${metacharacter_destination} to PATH before running 'cargo hawk'"
if [[ "${metacharacter_missing_output}" != "${expected_metacharacter_warning}" ]]; then
  echo "expected unmatched PATH metacharacters to emit a warning" >&2
  exit 1
fi
