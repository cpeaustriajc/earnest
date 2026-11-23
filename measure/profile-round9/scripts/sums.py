# Prints the summaries of time_turn.py / time_runs.py logs in short.  Usage: python3 sums.py LOG...
import sys, json
for p in sys.argv[1:]:
    for l in open(p):
        if '"summary"' not in l: continue
        d = json.loads(l)
        print(d['build'], d['model'], d['jobs'], 'wall', d['wall'], 'cpu', d['cpu'], 'sys', d['sys'][0], 'mib', d['mib'][0],
              'flt', (d.get('minflt') or [None])[0], 't', d.get('time_s'), 'rc', d.get('run_cpu_s'), d['statuses'])
