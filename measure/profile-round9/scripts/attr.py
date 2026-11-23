# Groups the samples of a perf recording by what the code does.
# Input: the output of
#   perf script -i X.data -F tid,ip,sym,dso,dsoff        (recorded with -g, frame pointers)
# and the debug binary. For a sample in the earnest binary, the sampled address is resolved with
# `addr2line -i -f -C` into its chain of inlined frames (innermost first); the group is decided by
# the innermost frame that lies in an earnest crate or in hashbrown (frames of core, alloc and std
# are skipped), as described in RESULTS.md. Kernel samples are grouped by the kernel frames of
# their call chain. Prints JSON: totals per group and subgroup, and the heaviest symbols.
import collections, json, re, subprocess, sys

script_out, binary = sys.argv[1], sys.argv[2]

samples = []  # (dso, dsoff, sym, kernel_chain_syms)
cur = None
with open(script_out, errors="replace") as fh:
    for line in fh:
        if not line.strip():
            if cur:
                samples.append(cur)
            cur = None
            continue
        if not line.startswith("\t"):
            if cur:
                samples.append(cur)
            cur = {"chain": []}
            continue
        # chain line: "\t    ffffffff81234 sym+0x12 (dso+0x123)"  or with dsoff
        m = re.match(r"\s*([0-9a-f]+)\s+(.*?)\s+\((.*)\)\s*$", line)
        if not m or cur is None:
            continue
        ip, sym, where = m.groups()
        sym = re.sub(r"\+0x[0-9a-f]+$", "", sym)
        cur["chain"].append((ip, sym, where))
if cur:
    samples.append(cur)

def dso_of(where):
    m = re.match(r"(.*)\+0x([0-9a-f]+)$", where)
    if m:
        return m.group(1), int(m.group(2), 16)
    return where, None

# Resolve every distinct offset in the earnest binary once.
offs = set()
for s in samples:
    if not s["chain"]:
        continue
    ip, sym, where = s["chain"][0]
    dso, off = dso_of(where)
    s["dso"], s["off"], s["sym"] = dso, off, sym
    if dso.endswith("earnest-sym") and off is not None:
        offs.add(off)
    if "kernel" in dso:
        for _, _, w in s["chain"]:
            d2, o2 = dso_of(w)
            if "kernel" not in d2:
                s["user_where"] = (d2, o2)
                if d2.endswith("earnest-sym") and o2 is not None:
                    offs.add(o2)
                break
offs = sorted(offs)
frames = {}
if offs:
    # perf's dsoff is a file offset; addr2line wants a virtual address: map through the LOAD segments.
    loads = []
    for l in subprocess.run(["readelf", "-lW", binary], capture_output=True, text=True).stdout.splitlines():
        f = l.split()
        if f and f[0] == "LOAD":
            loads.append((int(f[1], 16), int(f[2], 16), int(f[4], 16)))
    def vaddr(o):
        for off, va, size in loads:
            if off <= o < off + size:
                return o - off + va
        return o
    p = subprocess.run(["addr2line", "-i", "-f", "-C", "-a", "-e", binary],
                       input="\n".join(hex(vaddr(o)) for o in offs) + "\n", capture_output=True, text=True)
    back = {vaddr(o): o for o in offs}
    cur_off = None
    lines = p.stdout.splitlines()
    i = 0
    while i < len(lines):
        l = lines[i]
        if l.startswith("0x") and re.fullmatch(r"0x[0-9a-f]+", l):
            cur_off = back.get(int(l, 16), int(l, 16))
            frames[cur_off] = []
            i += 1
            continue
        fn = l
        loc = lines[i + 1] if i + 1 < len(lines) else "??:0"
        frames[cur_off].append((fn, loc))
        i += 2

EARNEST = re.compile(r"/crates/(earnest_[a-z_]+)/src/(.*?):\d+")
HASHBROWN = re.compile(r"/hashbrown-[^/]+/src/")

def file_group(crate, path, fn):
    if crate == "earnest_check":
        if path in ("store/layout.rs",) and re.search(r"\b(mix|hash_words|hash_each)\b", fn):
            return "e", "hash: store/layout.rs mix/hash_words"
        if path == "store/index.rs":
            return "a", "index: store/index.rs"
        if path in ("store/rows.rs", "store/codec.rs", "store/layout.rs"):
            return "b", "rows: " + path
        if path == "store.rs":
            if re.search(r"read_state|values_of|wide_state|encode|state\b", fn):
                return "b", "rows: store.rs " + short(fn)
            if re.search(r"\bfind\b|place\b|claim", fn):
                return "a", "index: store.rs " + short(fn)
            return "b", "rows: store.rs " + short(fn)
        if path == "explore/part.rs" and re.search(r"RecentWords", fn):
            # 89de199: a part's table of one-word rows is RecentWords, an array, not hashbrown; it is the
            # probe of a part's table, the same piece as "hashbrown in explore/part.rs" before
            return "c", "part table: RecentWords in explore/part.rs"
        if path in ("explore/part.rs", "explore/merge.rs", "store/level.rs"):
            return "c", "parts/claim: " + path
        if path in ("expand.rs", "explore/encode.rs", "explore/choice.rs"):
            return "d", "eval: " + path
        if path in ("explore/workers.rs", "explore.rs", "helpers.rs", "explore/here.rs"):
            return "g", "loop: " + path
        return "g", "other earnest_check: " + path
    if crate in ("earnest_ir", "earnest_value"):
        # the group is the crate's, as before; the subgroup now names the file (code/chain.rs is new at 3058cf4)
        return "d", "eval: " + crate + "/" + path
    return "g", "other: " + crate + "/" + path

def short(fn):
    fn = re.sub(r"<[^<>]*>", "", fn)
    fn = re.sub(r"<[^<>]*>", "", fn)
    return fn.split("::")[-1] if "::" in fn else fn

def classify_user(off, sym):
    ch = frames.get(off) or []
    # ch: innermost first
    hb = None
    for idx, (fn, loc) in enumerate(ch):
        m = EARNEST.search(loc)
        if m:
            crate, path = m.group(1), m.group(2)
            g, sub = file_group(crate, path, fn)
            if hb is not None:
                # a hashbrown operation inlined into this earnest frame
                if g == "e":
                    return g, sub
                if path in ("store/index.rs", "store.rs") or (path == "store/level.rs"):
                    return "a", "index: hashbrown in " + path
                if path == "explore/part.rs":
                    return "c", "part table: hashbrown in explore/part.rs"
                return g, "hashbrown in " + path
            return g, sub
        if HASHBROWN.search(loc) and hb is None:
            hb = idx
        if "rustc_hash" in loc or "/fxhash" in loc:
            return "e", "hash: rustc_hash"
    # no earnest frame: an outlined function of a library
    name = " ".join(f for f, _ in ch) + " " + sym
    if hb is not None or "hashbrown" in name:
        if re.search(r"Key<u32>|Key<u64>|NarrowId|WideId|Shards", name):
            return "a", "index: hashbrown outlined (" + short(ch[-1][0] if ch else sym) + ")"
        if re.search(r"HashTable<u64>|HashTable<usize>|RawTable<u64|RawTable<usize", name):
            return "c", "part table: hashbrown outlined (" + short(ch[-1][0] if ch else sym) + ")"
        return "a?", "hashbrown outlined, table not told: " + (ch[-1][0] if ch else sym)[:120]
    if re.search(r"alloc::raw_vec|__rust_alloc|__rust_realloc|__rdl_|alloc::alloc|drop_in_place", name):
        return "f", "alloc: Rust side (raw_vec, drop)"
    if re.search(r"std::thread|std::sys|parking|Mutex|futex|Condvar|Once", name):
        return "g", "threads/locks: std"
    # frames without line information: decide by the function's own name
    for pat, g, sub in ((r"earnest_ir::|earnest_value::|earnest_check::expand", "d", "eval (by name)"),
                        (r"PartSink|write_level|Winners|claim_level|merge", "c", "parts/claim (by name)"),
                        (r"read_state|Rows>::|codec", "b", "rows (by name)"),
                        (r"Shards|store::index", "a", "index (by name)"),
                        (r"explore::workers|Search>::work|Search>::level", "g", "loop (by name)")):
        if re.search(pat, name):
            return g, sub
    if re.search(r"memcpy|memmove|memset", name):
        return "g", "copy: " + sym
    return "g", "other user: " + (ch[-1][0] if ch else sym)[:120]

def classify_kernel(s):
    names = [c[1] for c in s["chain"]]
    joined = " ".join(names)
    if re.search(r"page_fault|handle_mm_fault|do_anonymous_page|clear_page", joined):
        return "f", "kernel: page faults"
    if re.search(r"munmap|mmap|madvise|brk|mremap|mprotect|vm_area|unmap_|free_pages|release_pages|zap_", joined):
        return "f", "kernel: mmap/munmap/madvise"
    if re.search(r"futex", joined):
        return "g", "kernel: futex (lock wait / wake)"
    if re.search(r"clone|copy_process|exit|wake_up_new_task|execve|load_elf", joined):
        return "g", "kernel: thread start/exit, exec"
    if re.search(r"sched_yield|schedule|finish_task_switch", joined):
        return "g", "kernel: scheduling"
    if re.search(r"read|write|open|close|stat|getdents", joined):
        return "g", "kernel: file i/o"
    return "g", "kernel: other (" + names[0] + ")"

LIBC_ALLOC = re.compile(r"^(malloc|realloc|free|cfree|_int_malloc|_int_free|_int_realloc|malloc_consolidate|"
                        r"__libc_malloc|__libc_free|__libc_realloc|sysmalloc|unlink_chunk|_int_free_merge_chunk|"
                        r"_int_free_create_chunk|_int_free_maybe_consolidate|tcache_.*|__GI___libc_.*|mremap_chunk|"
                        r"alloc_perturb|calloc|__libc_calloc|aligned_alloc|posix_memalign|_mid_memalign)$")

groups = collections.Counter()
ksites = collections.Counter()
subs = collections.Counter()
syms = collections.Counter()
symgroup = {}
total = 0
for s in samples:
    if not s.get("chain"):
        continue
    total += 1
    dso, sym = s["dso"], s["sym"]
    if "kernel" in dso:
        g, sub = classify_kernel(s)
        key = "[k] " + sym
        uw = s.get("user_where")
        if uw and uw[0].endswith("earnest-sym"):
            site = classify_user(uw[1], "")[1]
        elif uw:
            site = uw[0].split("/")[-1]
        else:
            site = "(no user frame)"
        ksites[(sub, site)] += 1
    elif dso.endswith("earnest-sym"):
        g, sub = classify_user(s["off"], sym)
        key = sym
    elif "libc" in dso or "ld-linux" in dso:
        if LIBC_ALLOC.match(sym):
            g, sub = "f", "alloc: libc malloc/realloc/free"
        elif re.search(r"mem(cpy|move|set)", sym) and len(s["chain"]) > 1 and s["chain"][1][1] in ("realloc", "_int_realloc", "__libc_realloc"):
            g, sub = "f", "alloc: libc memmove inside realloc"
        elif re.search(r"mem(cpy|move|set)", sym):
            g, sub = "g", "libc memmove/memset (caller not resolved)"
        elif re.search(r"pthread|futex|lll_|sem_|clone|start_thread", sym):
            g, sub = "g", "threads/locks: libc " + sym
        else:
            g, sub = "g", "libc other: " + sym
        key = sym + " (libc)"
    else:
        g, sub = "g", "other dso: " + dso
        key = sym + " (" + dso + ")"
    groups[g] += 1
    subs[(g, sub)] += 1
    syms[key] += 1
    symgroup.setdefault(key, collections.Counter())[g] += 1

out = {"total": total,
       "groups": {g: n for g, n in groups.most_common()},
       "subgroups": [[g, sub, n] for (g, sub), n in subs.most_common()],
       "kernel_sites": [[a, b, n] for (a, b), n in ksites.most_common(25)],
       "symbols": [[k, n, dict(symgroup[k])] for k, n in syms.most_common(40)]}
print(json.dumps(out, indent=1))
