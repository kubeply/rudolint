#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/install-hawk-path.sh
source "${script_directory}/install-hawk-path.sh"

temporary_directory="$(mktemp -d)"
stdout_path="${temporary_directory}/stdout"
stderr_path="${temporary_directory}/stderr"

cleanup() {
  rm -f "${stdout_path}" "${stderr_path}"
  rmdir "${temporary_directory}" 2>/dev/null || true
}
trap cleanup EXIT

capture_path_check() {
  local configured_path="$1"
  local destination="$2"

  PATH="${configured_path}" warn_if_hawk_bin_not_on_path "${destination}" \
    >"${stdout_path}" 2>"${stderr_path}"
  captured_stdout="$(<"${stdout_path}")"
  captured_stderr="$(<"${stderr_path}")"
}

assert_silent_when_present() {
  local destination="$1"

  capture_path_check "/usr/bin:${destination}:/bin" "${destination}"
  if [[ -n "${captured_stdout}" || -n "${captured_stderr}" ]]; then
    echo "expected no output when ${destination} is on PATH" >&2
    exit 1
  fi
}

assert_stderr_warning_when_missing() {
  local destination="$1"
  local configured_path="$2"
  local expected_warning="warning: add ${destination} to PATH before running 'cargo hawk'"

  capture_path_check "${configured_path}" "${destination}"
  if [[ -n "${captured_stdout}" ]]; then
    echo "expected no stdout when ${destination} is missing from PATH" >&2
    exit 1
  fi
  if [[ "${captured_stderr}" != "${expected_warning}" ]]; then
    echo "expected a stderr warning when ${destination} is missing from PATH" >&2
    exit 1
  fi
}

destination="/opt/rudolint-test-cargo/bin"
assert_silent_when_present "${destination}"
assert_stderr_warning_when_missing "${destination}" "/usr/bin:/bin"

metacharacter_destination="/opt/rudolint-test[01]*?/bin"
assert_silent_when_present "${metacharacter_destination}"
assert_stderr_warning_when_missing \
  "${metacharacter_destination}" \
  "/usr/bin:/opt/rudolint-test0-other/bin:/bin"
