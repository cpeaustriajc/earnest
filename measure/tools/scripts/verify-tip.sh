#!/bin/bash
N=/SCRATCH
T=/WORKTREES/perf-joined
W=$N/rows
L=$N/loop
NAME=${1:?name of this check}
BREAKS=${2:?runner of the deliberate breaks}
cd "$T" || exit 1
echo "tree: $(git rev-parse --short HEAD) $(git status --short | wc -l | tr -d ' ') changed files"
echo "== lint with Rust 1.100.0, the version of the check job"
( cargo --version; cargo fmt --version; cargo fmt --check >/dev/null 2>&1; echo "fmt exit $?"; cargo clippy --version; cargo clippy --all-targets --locked -- -D warnings >/dev/null 2>&1; echo "clippy exit $?" )
echo "== tests_kept"
python3 -I /REPO/.claude/skills/review-loop/scripts/tests_kept.py --base origin/main > $L/tests_kept_$NAME.txt 2>&1; echo "exit $? lines $(grep -c 'changes or removes base line' $L/tests_kept_$NAME.txt)"
diff <(grep 'changes or removes' $N/pr/tests_kept_loop1s.txt) <(grep 'changes or removes' $L/tests_kept_$NAME.txt) && echo "same 46 lines as allowed"
echo "== just check"
just check > $L/just-check-$NAME.log 2>&1; echo "just check exit $?"
python3 -I -c '
import re,sys
t=open(sys.argv[1]).read()
print("rust tests passed:", sum(int(m) for m in re.findall(r"test result: ok\. (\d+) passed", t)), "failed:", sum(int(m) for m in re.findall(r"(\d+) failed", t)))
' $L/just-check-$NAME.log
tail -3 $L/just-check-$NAME.log
echo "== the two tests with a time limit of 1 s, 8 runs each"
for t in limits::a_time_stop_in_the_last_pass_keeps_every_state_judged_before_it_on_every_worker_count limits::a_time_stop_in_a_later_round_of_a_level_leaves_nothing_of_the_level_behind; do
  ok=0; for i in 1 2 3 4 5 6 7 8; do cargo test --locked -p earnest_check --test explore -- --exact "$t" 2>&1 | grep -q 'test result: ok. 1 passed' && ok=$((ok+1)); done; echo "$t: $ok of 8 pass"
done
uptime
echo "== comparison"
cargo build --release --locked -p earnest-cli 2>&1 | tail -1
cp target/release/earnest "$W/earnest-$NAME"
cd "$W" || exit 1
find cmp -name '*.base' -delete; find cmp -name '*.new' -delete; find cmp -name '*.diff' -delete
echo "left over: $(find cmp -name '*.base' | wc -l | tr -d ' ') base files"
sh ./compare.sh "$W/earnest-$NAME" > "$W/cmp-$NAME.txt" 2>&1
tail -6 "$W/cmp-$NAME.txt" | cut -c1-240
echo "base files made: $(find cmp -name '*.base' | wc -l | tr -d ' '), diff files: $(find cmp -name '*.diff' | wc -l | tr -d ' ')"
echo "== statuses of the new verify runs"
find cmp -name '*.new' -exec grep -ohE '"status": "[a-z]+"' {} + | sort | uniq -c | tr '\n' ' '; echo
echo "== sweep of the state limit against the 53d8497 build"
for m in c1 big4 l1; do TIP="$W/earnest-$NAME" python3 -I $L/sweep.py $N/review5/models/$m.model.ts; done
echo "== instructions"
bash $L/measure-join.sh "$W/earnest-$NAME" $NAME 2>&1 | cut -c1-120
echo "== deliberate breaks, each in a copy of the tree"
bash "$BREAKS"
cd "$T"; echo "tree after: $(git rev-parse --short HEAD) $(git status --short | wc -l | tr -d ' ') changed files"
