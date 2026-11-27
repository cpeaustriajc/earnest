#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
quint="$QUINT_PACKAGE/../../.bin/quint"
earnest="$1"
folder="$2"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
cat > "$scratch/quint" <<WRAPPER
#!/usr/bin/env bash
if [ "\$1" = compile ]; then (IFS=\$'\\t'; printf '%s\n' "\$*") >> "$scratch/compiled.txt"; fi
if [ "\$1" = typecheck ]; then echo "no typecheck: every check is compiled" >&2; exit 1; fi
exec "$quint" "\$@"
WRAPPER
chmod +x "$scratch/quint"
status=0
(cd "$scratch" && EARNEST_QUINT="$scratch/quint" "$earnest" verify "$folder" > /dev/null 2>&1) || status=$?
if [ "$status" -gt 1 ]; then
  echo "earnest verify stopped with status $status, not the 1 it gives when a model fails" >&2
  exit "$status"
fi
sed "s#$root/##" "$scratch/compiled.txt" | sort -u
