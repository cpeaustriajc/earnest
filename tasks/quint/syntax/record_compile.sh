#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
out="$1"
checks="$2"
quint="$QUINT_PACKAGE/../../.bin/quint"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
cd "$root"
count=0
while IFS= read -r line; do
  [ -z "$line" ] && continue
  count=$((count + 1))
  printf '%s\n' "$line" > "$scratch/$count.args"
  read -r -a words <<< "$line"
  "$quint" "${words[@]}" > "$scratch/$count.out" 2> "$scratch/$count.err" || true
done < "$checks"
python3 -I "$here/condense_compile.py" "$root" "$out" "$scratch"
