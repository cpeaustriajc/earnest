#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
out="$1"
shift
quint="$QUINT_PACKAGE/../../.bin/quint"
for file in "$@"; do
  relative="$(realpath --relative-to="$root" "$file")"
  target="$out/$relative"
  mkdir -p "$(dirname "$target")"
  node "$here/phases.js" "$file" "$target.phases.json"
  "$quint" parse "$file" --out "$target.parse.json" --source-map "$target.map.json" > /dev/null 2>&1 || true
  python3 -I "$here/condense.py" "$root" "$target.phases.json" "$target.parse.json" "$target.map.json"
  rm "$target.map.json"
  "$here/record_types.sh" "$out" "$file"
done
