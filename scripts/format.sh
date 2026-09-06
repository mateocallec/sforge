#!/usr/bin/env bash
# Formats cli/ (rustfmt) and/or extension/ (prettier).
#
# Usage:
#   scripts/format.sh              # both
#   scripts/format.sh cli          # cli/ only
#   scripts/format.sh extension    # extension/ only

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

format_cli() {
  echo "==> rustfmt (cli/)"
  (cd "$root_dir/cli" && cargo fmt --all)
}

format_extension() {
  echo "==> prettier (extension/)"
  (cd "$root_dir/extension" && npm run --silent format)
}

if [[ "$target" == "all" || "$target" == "cli" ]]; then
  format_cli
fi

if [[ "$target" == "all" || "$target" == "extension" ]]; then
  format_extension
fi
