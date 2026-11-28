#!/bin/bash
from=$1; to=$2; out=$3; mkdir -p $out
E=${EARNEST:-earnest}
for s in $(seq $from $to); do
  timeout 20 python3 "$(dirname "$0")"/gen8.py $s $out 2>/dev/null || { echo "$s generror"; continue; }
  (cd $out && $E verify --json --no-cache g$s.qnt > g$s.json 2>/dev/null)
  python3 - $out $s <<'PY'
import json,sys
out,s=sys.argv[1],sys.argv[2]
d=json.load(open(f"{out}/g{s}.json"))
exp=dict((l.split()[0], l.split()[1]) for l in open(f"{out}/g{s}.expect") if l.strip())
if not d['models']:
    u=d.get('unreadable')
    print(s,"UNREADABLE", json.dumps(u)[:300]); sys.exit()
m=d['models'][0]
got={r['id']:r['verdict'] for r in m['rules']}
for k,v in exp.items():
    g=got.get(k,'missing')
    if g!=v: print(s,k,"oracle",v,"earnest",g)
PY
done
