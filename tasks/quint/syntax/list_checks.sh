#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
folder="${1:-tasks}"
list="$(mktemp)"
trap 'rm -f "$list"' EXIT
(cd "$root" && EARNEST_LISTED_FOLDER="$folder" EARNEST_LISTED_CHECKS="$list" \
  cargo test --locked -q -p earnest-cli --bin earnest -- --ignored \
  quint::tests::the_checks_earnest_compiles_are_listed_as_quint_compile_is_called > /dev/null)
cat "$list"
