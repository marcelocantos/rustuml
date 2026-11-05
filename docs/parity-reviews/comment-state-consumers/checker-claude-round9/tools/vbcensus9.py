"""Exact viewBox and comment-list census per batch and tree. Usage: vbcensus9.py <work> <out.json> <labels...>"""
import json, re, sys, pathlib
W = pathlib.Path(sys.argv[1]); out = {}; C = re.compile(r"<!--(.*?)-->", re.S); V = re.compile(r'viewBox="([^"]*)"')
for L in sys.argv[3:]:
    rows = json.load(open(W / f"batch-{L}.json")); O = W / f"out-{L}"; s = {}
    vb = {t: set() for t in ["base", "merge", "tip"]}; cm = {t: set() for t in ["base", "merge", "tip"]}
    for n, r in rows.items():
        if r["format"] != "-tsvg": continue
        j = (O / f"{n}.java.svg").read_text(errors="replace"); jv = V.search(j); jc = C.findall(j)
        for t in vb:
            x = (O / f"{n}.{t}.svg").read_text(errors="replace"); xv = V.search(x)
            if jv and xv and jv.group(1) == xv.group(1): vb[t].add(n)
            if C.findall(x) == jc: cm[t].add(n)
    s["exact_viewbox_base_merge_tip"] = [len(vb[t]) for t in vb]; s["comment_list_exact_base_merge_tip"] = [len(cm[t]) for t in cm]
    s["viewbox_lost_merge_to_tip"] = sorted(vb["merge"] - vb["tip"]); s["viewbox_lost_base_to_tip"] = sorted(vb["base"] - vb["tip"])
    s["comment_list_lost_merge_to_tip"] = sorted(cm["merge"] - cm["tip"]); s["comment_list_lost_base_to_tip"] = sorted(cm["base"] - cm["tip"])
    out[L] = s; print(L, s)
json.dump(out, open(sys.argv[2], "w"), indent=1)
