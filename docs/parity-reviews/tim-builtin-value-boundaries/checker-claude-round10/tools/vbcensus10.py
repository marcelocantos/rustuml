"""Exact viewBox and comment-list census per relay batch and tree; lists lost merge->tip and base->tip.
Usage: python3 -I vbcensus10.py <work> <out.json> <labels...>"""
import json, pathlib, re, sys
W = pathlib.Path(sys.argv[1]); out = {}
VB = re.compile(r'viewBox="([^"]*)"'); CM = re.compile(r"<!--(.*?)-->", re.S)
for L in sys.argv[3:]:
    rows = json.load(open(W / f"batch-{L}.json")); o = W / f"out-{L}"; res = {"cases": len(rows)}
    vb = {}; cm = {}
    for n, r in rows.items():
        if r["format"] != "-tsvg": continue
        j = (o / f"{n}.java.svg").read_text(errors="replace")
        for t in ("base", "merge", "tip"):
            a = (o / f"{n}.{t}.svg").read_text(errors="replace")
            vb[(n, t)] = VB.findall(a) == VB.findall(j); cm[(n, t)] = CM.findall(a) == CM.findall(j)
    names = sorted({n for n, _ in vb})
    for t in ("base", "merge", "tip"):
        res[f"viewbox_exact_{t}"] = sum(vb[(n, t)] for n in names); res[f"comments_exact_{t}"] = sum(cm[(n, t)] for n in names)
    res["viewbox_lost_merge_to_tip"] = [n for n in names if vb[(n, "merge")] and not vb[(n, "tip")]]
    res["viewbox_lost_base_to_tip"] = [n for n in names if vb[(n, "base")] and not vb[(n, "tip")]]
    res["comments_lost_merge_to_tip"] = [n for n in names if cm[(n, "merge")] and not cm[(n, "tip")]]
    res["comments_lost_base_to_tip"] = [n for n in names if cm[(n, "base")] and not cm[(n, "tip")]]
    out[L] = res
    print(L, {k: (len(v) if isinstance(v, list) else v) for k, v in res.items()})
json.dump(out, open(sys.argv[2], "w"), indent=1)
