#!/usr/bin/env bash
# Task 4: the JSON of grid_10m with --jobs 4, `milliseconds` lines removed, for the tip and each prototype build.
# Run from the repository root.
R=measure/round6/raw
for b in tip p16 p8 p32; do
  /tmp/bin/earnest-$b verify --no-cache --json --jobs 4 --max-states 200000000 tasks/benchmark/models/grid_10m.model.ts > $R/task4_json_$b.json
  echo "build=$b exit=$?"
  grep -v '"milliseconds"' $R/task4_json_$b.json > $R/task4_json_${b}_no_ms.json
done
for b in p16 p8 p32; do
  if cmp -s $R/task4_json_tip_no_ms.json $R/task4_json_${b}_no_ms.json; then echo "tip vs $b: same"; else echo "tip vs $b: DIFFERENT"; diff $R/task4_json_tip_no_ms.json $R/task4_json_${b}_no_ms.json; fi
done
