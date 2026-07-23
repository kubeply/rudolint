#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/install-hawk-path.sh
source "${script_directory}/install-hawk-path.sh"

hawk_version="0.1.9"
release_base_url="https://github.com/astral-sh/hawk/releases/download/${hawk_version}"

case "$(uname -s):$(uname -m)" in
  Darwin:arm64)
    target="aarch64-apple-darwin"
    archive_sha256="0699ef6d4134a1b6902445c5162ce273b76b3ffe7cb451da7b5d359084f87237"
    ;;
  Darwin:x86_64)
    target="x86_64-apple-darwin"
    archive_sha256="d42252dafc94aa741a5d559f67b3016d8d7faa57f5cc765fb71fac3f7c1f1ae9"
    ;;
  Linux:aarch64 | Linux:arm64)
    target="aarch64-unknown-linux-gnu"
    archive_sha256="bf2b31d180e7716eb69134c4340d6a708b82f16570c7e06c2dd68664db4ab438"
    ;;
  Linux:x86_64 | Linux:amd64)
    target="x86_64-unknown-linux-gnu"
    archive_sha256="027124444baddf7fa3597ce2d76d1eff48e95902be303beada2558e3687c1bff"
    ;;
  *)
    echo "unsupported Hawk platform: $(uname -s) $(uname -m)" >&2
    exit 1
    ;;
esac

archive_name="cargo-hawk-${target}.tar.gz"
temporary_directory="$(mktemp -d)"
archive_path="${temporary_directory}/${archive_name}"
archive_root="cargo-hawk-${target}"

cleanup() {
  rm -f \
    "${archive_path}" \
    "${temporary_directory}/cargo-hawk" \
    "${temporary_directory}/cargo-hawk-driver"
  rmdir "${temporary_directory}" 2>/dev/null || true
}
trap cleanup EXIT

curl --proto '=https' --tlsv1.2 -LsSf \
  "${release_base_url}/${archive_name}" \
  --output "${archive_path}"

if command -v sha256sum >/dev/null 2>&1; then
  checksum_output="$(sha256sum "${archive_path}")"
elif command -v shasum >/dev/null 2>&1; then
  checksum_output="$(shasum --algorithm 256 "${archive_path}")"
else
  echo "Hawk installation requires sha256sum or shasum" >&2
  exit 1
fi

actual_sha256="${checksum_output%% *}"
if [[ "${actual_sha256}" != "${archive_sha256}" ]]; then
  echo "Hawk archive checksum mismatch for ${archive_name}" >&2
  exit 1
fi

tar -xzf "${archive_path}" \
  -C "${temporary_directory}" \
  --strip-components=1 \
  "${archive_root}/cargo-hawk" \
  "${archive_root}/cargo-hawk-driver"

if [[ -n "${CARGO_HOME:-}" ]]; then
  cargo_home="${CARGO_HOME}"
elif [[ -n "${HOME:-}" ]]; then
  cargo_home="${HOME}/.cargo"
else
  echo "Hawk installation requires CARGO_HOME or HOME" >&2
  exit 1
fi

destination="${cargo_home}/bin"
mkdir -p "${destination}"
install -m 0755 "${temporary_directory}/cargo-hawk" "${destination}/cargo-hawk"
install -m 0755 "${temporary_directory}/cargo-hawk-driver" "${destination}/cargo-hawk-driver"

echo "installed Hawk ${hawk_version} for ${target} to ${destination}"
warn_if_hawk_bin_not_on_path "${destination}"
