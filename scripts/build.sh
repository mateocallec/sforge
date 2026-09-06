#!/usr/bin/env bash
# Compiles cli/ (cargo build) and/or extension/ (npm run build).
#
# Usage:
#   scripts/build.sh              # both
#   scripts/build.sh cli          # cli/ only
#   scripts/build.sh extension    # extension/ only

set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="${1:-all}"

case "$target" in
  all | cli | extension) ;;
  *)
    echo "usage: $(basename "$0") [cli|extension]" >&2
    exit 1
    ;;
esac

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: '$1' is required but not found on PATH" >&2
    exit 1
  fi
}

build_cli() {
  echo "==> cargo build (cli/)"
  require cargo
  (cd "$root_dir/cli" && cargo build)
}

build_extension() {
  echo "==> npm run build (extension/)"
  require npm
  if [[ ! -d "$root_dir/extension/node_modules" ]]; then
    echo "    node_modules missing, running npm ci first"
    (cd "$root_dir/extension" && npm ci)
  fi
  (cd "$root_dir/extension" && npm run build)
}

if [[ "$target" == "all" || "$target" == "cli" ]]; then
  build_cli
fi

if [[ "$target" == "all" || "$target" == "extension" ]]; then
  build_extension
fi
