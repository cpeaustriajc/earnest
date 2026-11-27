#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
out="$1"
shift
quint="$QUINT_PACKAGE/../../.bin/quint"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
count=0
while IFS= read -r line; do
  [ -z "$line" ] && continue
  count=$((count + 1))
  printf '%s\n' "$line" > "$scratch/$count.args"
done < <(cat "$@")
cd "${RECORD_FROM:-$root}"
export quint scratch
seq 1 "$count" | xargs -P 4 -I {} bash -c '
  IFS=$'\''\t'\'' read -r -a words < "$scratch/{}.args"
  "$quint" "${words[@]}" > "$scratch/{}.out" 2> "$scratch/{}.err" || true
'
python3 -I "$here/condense_compile.py" "$root" "$out" "$scratch"
