#!/usr/bin/env bash

warn_if_hawk_bin_not_on_path() {
  local destination="$1"

  case ":${PATH:-}:" in
    *":${destination}:"*) ;;
    *)
      echo "warning: add ${destination} to PATH before running 'cargo hawk'" >&2
      ;;
  esac
}
