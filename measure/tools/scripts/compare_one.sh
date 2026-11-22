#!/bin/sh
W=/SCRATCH/rows
REF=/SCRATCH/joined/earnest-joined
NEW=$1; f=$2; mode=$3; j=$4
case $mode in
  verify) args="verify --no-cache --json --max-states 20000000";;
  verify50) args="verify --no-cache --json --max-states 50";;
  verify7) args="verify --no-cache --json --max-states 7";;
  report) args="report --max-arrows 100000";;
  report50) args="report --max-arrows 100000 --max-states 50";;
esac
key=$W/cmp/$(echo "$f.$mode" | tr '/' '_')
strip() { python3 -c '
import sys,re
t=sys.stdin.read()
t=re.sub(r"\"milliseconds\": [0-9]+","\"milliseconds\": 0",t)
t=re.sub(r"[0-9]+(\.[0-9]+)? ?(ms|s)\b","T",t)
print(t)'; }
cd $W
[ -s "$key.$j.base" ] || $REF $args --jobs $j "$f" 2>&1 | strip > "$key.$j.base"
"$NEW" $args --jobs $j "$f" 2>&1 | strip > "$key.$j.new"
echo "RUN" 
cmp -s "$key.$j.base" "$key.$j.new" || { diff "$key.$j.base" "$key.$j.new" > "$key.$j.diff"; echo "DIFF ref $f $mode $j"; }
