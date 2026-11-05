"""Changed outputs between two sweep trees, each judged by the unchanged strict comparator against the
repo reference .svg. Usage: quickdiff.py <work> <repo> <treeA> <treeB>"""
import json, pathlib, subprocess, sys, collections
W, R, A, B = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3], sys.argv[4]
CMP = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round5-claude-maker/cmp/target/release/checker-cmp"
sa = json.load(open(W / f"sweep-{A}/status.json")); sb = json.load(open(W / f"sweep-{B}/status.json"))
ch = sorted(k for k in sb if k in sa and sa[k]["stdout_sha256"] != sb[k]["stdout_sha256"])
lst = W / f"qd-{A}-{B}.lst"
lst.write_text("".join(f"{(R / k).with_suffix('.svg')}\t{(W / f'sweep-{t}' / k).with_suffix('.svg')}\n" for k in ch for t in (A, B)))
out = subprocess.run([str(CMP), str(lst)], capture_output=True, text=True).stdout.splitlines() if ch else []
res = collections.Counter(); rows = []
for i, k in enumerate(ch):
    ra, rb = out[2 * i].split("\t"), out[2 * i + 1].split("\t")
    key = f"{ra[1]}->{rb[1]}"; res[key] += 1
    rows.append((k, key, ra[2], rb[2], sa[k]["status"], sb[k]["status"]))
print("changed", len(ch), dict(res))
for r in rows: print(*r)
