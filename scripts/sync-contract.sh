#!/usr/bin/env bash
# Copies TaskHub's API contract (OpenAPI document and fixtures) into api/v1/ and records where it came from.
# The snapshot is generated: never edit api/v1/ by hand. Usage: scripts/sync-contract.sh <path-to-TaskHub>
set -euo pipefail

taskhub=${1:?usage: scripts/sync-contract.sh <path-to-TaskHub>}
source_dir="$taskhub/docs/api/v1"
root=$(cd "$(dirname "$0")/.." && pwd)
target="$root/api/v1"

[[ -f "$source_dir/openapi.json" && -d "$source_dir/fixtures" ]] || { echo "No API contract in $source_dir" >&2; exit 2; }
commit=$(git -C "$taskhub" rev-parse HEAD)
if [[ -n $(git -C "$taskhub" status --porcelain -- docs/api/v1) ]]; then
  echo "TaskHub's docs/api/v1 has uncommitted changes; commit them first." >&2
  exit 2
fi

rm -rf "$target"
mkdir -p "$target"
cp "$source_dir/openapi.json" "$target/"
cp -R "$source_dir/fixtures" "$target/"

cd "$target"
{
  printf '{\n  "taskhubCommit": "%s",\n  "files": {\n' "$commit"
  find . -type f ! -name contract-source.json | sed 's#^\./##' | LC_ALL=C sort | while read -r file; do
    printf '    "%s": "%s",\n' "$file" "$(sha256sum "$file" | cut -d' ' -f1)"
  done | sed '$ s/,$//'
  printf '  }\n}\n'
} > contract-source.json
echo "Synced the API contract from TaskHub $commit"
