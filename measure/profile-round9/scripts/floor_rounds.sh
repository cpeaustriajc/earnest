#!/usr/bin/env bash
# Floors in turn: for each wheel count in the file, one uncounted run of its first line, then ROUNDS rounds,
# each round running every line of that wheel count once, in file order.
# CONF lines: "label wheels env... -- args..."  e.g.  "shared4 8 HUGE=1 CHUNK=256 DIST=8 -- shared 4"
# Usage: floor_rounds.sh CONF ROUNDS
S=${S:-/SCRATCH}; B=$S/bin; conf=$1; rounds=${2:-5}
echo "# sha256"; sha256sum $B/floor_par | sed 's/^/# /'
echo "# conf"; sed 's/^/# /' $conf
echo "# loadavg_before=$(cat /proc/loadavg) utc=$(date -u +%FT%TZ)"
one() { # label wheels round env... -- args...
  local label=$1 w=$2 r=$3; shift 3
  local envs=(); while [ "$1" != "--" ]; do envs+=("$1"); shift; done; shift
  echo "### run label=$label wheels=$w round=$r env='${envs[*]}' args='$*'"
  env "${envs[@]}" /usr/bin/time -f 'gnutime wall=%e user=%U sys=%S maxrss_kb=%M minflt=%R' $B/floor_par $w "$@" 2>&1
  echo "### end exit=$?"
}
for w in $(grep -v '^#' $conf | awk 'NF{print $2}' | awk '!s[$0]++'); do
  first=$(grep -v '^#' $conf | awk -v w=$w '$2==w' | head -1)
  one $(echo $first | awk '{print $1, $2}') 0 $(echo $first | cut -d' ' -f3-)
  for r in $(seq $rounds); do
    grep -v '^#' $conf | awk -v w=$w '$2==w' | while read -r label ww rest; do one $label $ww $r $rest; done
  done
done
echo "# loadavg_after=$(cat /proc/loadavg) utc=$(date -u +%FT%TZ)"
