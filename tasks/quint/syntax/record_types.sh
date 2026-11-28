#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
base="${RECORD_BASE:-$root}"
out="$1"
shift
quint="$QUINT_PACKAGE/../../.bin/quint"
for file in "$@"; do
  relative="$(realpath --relative-to="$base" "$file")"
  target="$out/$relative"
  mkdir -p "$(dirname "$target")"
  rm -f "$target.types.json" "$target.typecheck.json" "$target.inferred.json" "$target.parsed.json"
  node "$here/types.js" "$file" "$target.inferred.json"
  "$quint" typecheck "$file" --out "$target.typecheck.json" > /dev/null 2>&1 || true
  "$quint" parse "$file" --out "$target.parsed.json" > /dev/null 2>&1 || true
  python3 -I "$here/condense_types.py" "$root" "$target.typecheck.json" "$target.inferred.json" "$target.types.json" "$target.parsed.json"
  rm -f "$target.typecheck.json" "$target.inferred.json" "$target.parsed.json"
done
