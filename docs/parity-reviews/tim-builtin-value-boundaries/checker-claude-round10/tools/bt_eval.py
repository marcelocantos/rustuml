"""Evaluate BT (C10A) against checker-expectations.json. Usage: python3 -I bt_eval.py <batch-BT.json> <checker-expectations.json> <out.json>"""
import json, sys, collections
rows = json.load(open(sys.argv[1])); exp = json.load(open(sys.argv[2]))["cases"]
M = lambda n, t: rows[n][t + "_strict"] == "MATCH" and rows[n].get(t + "_comments_exact", True)
out = {"by_expect": {}, "by_group": {}, "violations": [], "losses": [], "gains": [], "base_losses": []}
for e in exp:
    n = e["name"]; g = n.split("_")[1]; tip, mer, base = M(n, "tip"), M(n, "merge"), M(n, "base")
    for key, d in (("by_expect", e["expect"]), ("by_group", g)):
        c = out[key].setdefault(d, collections.Counter()); c["n"] += 1; c["tip"] += tip; c["merge"] += mer; c["base"] += base
    if mer and not tip: out["losses"].append(n)
    if base and not tip: out["base_losses"].append(n)
    if tip and not mer: out["gains"].append(n)
    ctl = all(M(c, "tip") for c in e["controls"]) if e["controls"] else None
    if e["expect"] == "match" and not tip: out["violations"].append([n, "expected match", rows[n]["tip_ndiff"]])
    if e["expect"] == "fail" and tip: out["violations"].append([n, "expected fail, matched"])
    if e["expect"] == "match-iff-control" and ctl and not tip: out["violations"].append([n, "control matches, probe fails", rows[n]["tip_ndiff"], rows[n].get("tip_diff_head", "")[:220]])
    if e["expect"] == "match-iff-control" and not ctl and tip: out["violations"].append([n, "probe matches, control fails"])
for k in ("by_expect", "by_group"): out[k] = {a: dict(b) for a, b in out[k].items()}
json.dump(out, open(sys.argv[3], "w"), indent=1)
print(json.dumps(out["by_expect"])); print(json.dumps(out["by_group"])); print("losses", out["losses"], "base_losses", out["base_losses"]); print("gains", len(out["gains"]))
for v in out["violations"]: print(v)
