#!/usr/bin/env bash
set -uo pipefail

command -v just >/dev/null || { echo "Stop hook: just is not on PATH. Install it, then run just check." >&2; exit 2; }

hook_cwd=$(jq -r '.cwd // empty')
cd "$hook_cwd" 2>/dev/null && just --summary >/dev/null 2>&1 || cd "$CLAUDE_PROJECT_DIR" || exit 2

[ -n "$(git status --porcelain)" ] || exit 0

fingerprint=$({
  git rev-parse HEAD
  git diff HEAD --binary
  git ls-files --others --exclude-standard -z | xargs -0 shasum 2>/dev/null
} | shasum | cut -d' ' -f1)
last_pass_file=$(git rev-parse --git-path earnest-check-passed)

[ "$(cat "$last_pass_file" 2>/dev/null)" = "$fingerprint" ] && exit 0

out=$(just check 2>&1) || { printf '%s\n' "$out" | tail -n 80 >&2; exit 2; }
printf '%s\n' "$fingerprint" > "$last_pass_file"
