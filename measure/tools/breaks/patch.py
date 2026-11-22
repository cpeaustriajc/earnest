import os, sys
name, root = sys.argv[1], sys.argv[2]
W = "crates/earnest_check/src/explore/workers.rs"
E = "crates/earnest_check/src/explore.rs"
MUT = {
    "through_dropped": [(E, "        let merged = self.merge_any(expansion.parts, judged);",
                         "        let mut of_merge = Judged::default();\n        let merged = self.merge_any(expansion.parts, &mut of_merge);\n        judged.add(of_merge.marks, 0);")],
    "hold_after_swap_plain": [(W, """                let open = part < level.parts()
                    && !self.stopped.load(Ordering::SeqCst)
                    && !self.holds_back_at(level, part);
                open.then(|| part.saturating_add(1))
            })
            .ok()""", """                let open = part < level.parts()
                    && !self.stopped.load(Ordering::SeqCst);
                open.then(|| part.saturating_add(1))
            })
            .ok()
            .filter(|part| !self.holds_back_at(level, *part))""")],
    "merge_skips_a_task_on_ten_threads": [(W, "                if task >= tasks {", "                if task >= tasks || (self.own >= 10 && task + 1 == tasks) {")],
}
for path, old, new in MUT[name]:
    full = os.path.join(root, path); s = open(full).read(); assert s.count(old) == 1, (name, old[:60])
    open(full, "w").write(s.replace(old, new))
