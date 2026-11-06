"""Exact viewBox and comment-list census of the relay batches per tree.
Usage: python3 -I viewbox_census.py <work> <out.json> <label>...  (trees base, merge, tip)"""
import json, pathlib, re, sys
W = pathlib.Path(sys.argv[1]); V = re.compile(r'viewBox="([^"]*)"'); C = re.compile(r"<!--(.*?)-->", re.S)
out = {}
for L in sys.argv[3:]:
    rows = json.load(open(W / f"batch-{L}.json")); res = {"cases_exit0_svg": 0}
    per = {}
    for n, r in rows.items():
        if r["format"] != "-tsvg" or r["java_exit"] != 0: continue
        res["cases_exit0_svg"] += 1
        j = (W / f"out-{L}" / f"{n}.java.svg").read_text(errors="replace"); jv = V.search(j); jc = C.findall(j)
        row = {"java": jv.group(1) if jv else None}
        for t in ["base", "merge", "tip"]:
            a = (W / f"out-{L}" / f"{n}.{t}.svg").read_text(errors="replace"); av = V.search(a)
            row[t] = av.group(1) if av else None; row[f"{t}_comments_exact"] = C.findall(a) == jc
        per[n] = row
    for t in ["base", "merge", "tip"]:
        res[f"{t}_exact_viewbox"] = sum(r[t] == r["java"] for r in per.values())
        res[f"{t}_comments_exact"] = sum(r[f"{t}_comments_exact"] for r in per.values())
    for b in ["base", "merge"]:
        res[f"viewbox_lost_vs_{b}"] = sorted(n for n, r in per.items() if r[b] == r["java"] and r["tip"] != r["java"])
        res[f"viewbox_gained_vs_{b}"] = sorted(n for n, r in per.items() if r[b] != r["java"] and r["tip"] == r["java"])
        res[f"comments_lost_vs_{b}"] = sorted(n for n, r in per.items() if r[f"{b}_comments_exact"] and not r["tip_comments_exact"])
    res["tip_viewbox_mismatch"] = {n: [r["java"], r["tip"]] for n, r in per.items() if r["tip"] != r["java"]}
    out[L] = res
json.dump(out, open(sys.argv[2], "w"), indent=1)
for L, r in out.items():
    print(L, {k: (v if not isinstance(v, (list, dict)) else len(v)) for k, v in r.items()})
