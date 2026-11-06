"""Evaluate a three-tree batch: Java validity from raw SVG + stderr, lost/gained matches, rejections, errors, panics,
exact viewBox and comment lists, stderr; per-hypothesis tests for C11A. Usage: python3 -I eval11.py <batch.json> <out.json> [expectations.json]"""
import json, re, sys, pathlib, collections
rows = json.load(open(sys.argv[1])); W = pathlib.Path(sys.argv[1]).parent; LABEL = pathlib.Path(sys.argv[1]).stem.split("-", 1)[1]
exp = {r["name"]: r for r in json.load(open(sys.argv[3]))["cases"]} if len(sys.argv) > 3 else {}
VB = re.compile(r'viewBox="([^"]*)"')
def vb(path):
    m = VB.search(path.read_text(errors="replace")) if path.exists() else None; return m.group(1) if m else None
out = {"cases": len(rows), "trees": {}, "deltas": {}, "per_case": {}}
for n, r in rows.items():
    jvalid = r["java_exit"] == 0 and not r["java_error_marker"] and not r["java_crash"] and not r["java_stderr"]
    r["java_valid"] = jvalid
    jvb = vb(W / f"out-{LABEL}" / f"{n}.java.svg")
    pc = {"java_valid": jvalid, "java_stderr": r["java_stderr"], "java_viewBox": jvb}
    for t in ["base", "merge", "tip"]:
        pc[t] = {"strict": r[f"{t}_strict"], "ndiff": r[f"{t}_ndiff"], "exit": r[f"{t}_exit"], "panic": r[f"{t}_panic"],
                 "stderr": r[f"{t}_stderr"], "comments_exact": r.get(f"{t}_comments_exact"),
                 "viewBox_exact": vb(W / f"out-{LABEL}" / f"{n}.{t}.svg") == jvb, "sha256": r[f"{t}_sha256"]}
    out["per_case"][n] = pc
for t in ["base", "merge", "tip"]:
    pcs = out["per_case"].values()
    out["trees"][t] = {"java_valid": sum(p["java_valid"] for p in pcs),
                       "strict_match_valid": sum(p[t]["strict"] == "MATCH" for p in pcs if p["java_valid"]),
                       "viewBox_exact": sum(p[t]["viewBox_exact"] for p in pcs), "comments_exact": sum(bool(p[t]["comments_exact"]) for p in pcs),
                       "nonzero_exit": sum(p[t]["exit"] != 0 for p in pcs), "panics": sum(p[t]["panic"] for p in pcs),
                       "nonempty_stderr": sum(bool(p[t]["stderr"]) for p in pcs)}
for a, b in [("base", "tip"), ("merge", "tip"), ("base", "merge")]:
    ev = collections.defaultdict(list)
    for n, p in out["per_case"].items():
        if p["java_valid"]:
            if p[a]["strict"] == "MATCH" and p[b]["strict"] != "MATCH": ev["lost_match"].append(n)
            if p[b]["strict"] == "MATCH" and p[a]["strict"] != "MATCH": ev["gained_match"].append(n)
            if p[a]["exit"] == 0 and p[b]["exit"] != 0: ev["new_valid_input_error"].append(n)
        else:
            if p[a]["exit"] != 0 and p[b]["exit"] == 0: ev["lost_rejection"].append(n)
        if p[b]["panic"] and not p[a]["panic"]: ev["new_panic"].append(n)
        if p[a]["viewBox_exact"] and not p[b]["viewBox_exact"]: ev["viewBox_lost"].append(n)
        if p[b]["viewBox_exact"] and not p[a]["viewBox_exact"]: ev["viewBox_gained"].append(n)
        if p[a]["comments_exact"] and not p[b]["comments_exact"]: ev["comments_lost"].append(n)
        if p[a]["stderr"] != p[b]["stderr"]: ev["stderr_changed"].append(n)
        if p[a]["sha256"] != p[b]["sha256"]: ev["output_changed"].append(n)
        if p[a]["ndiff"] >= 0 and p[b]["ndiff"] > p[a]["ndiff"] and p[b]["strict"] != "MATCH": ev["ndiff_worse"].append(n)
    out["deltas"][f"{a}->{b}"] = {k: sorted(v) for k, v in ev.items()}
    out["deltas"][f"{a}->{b}_counts"] = {k: len(v) for k, v in ev.items()}
if exp:
    H = collections.defaultdict(lambda: collections.Counter()); fails = collections.defaultdict(list)
    for n, p in out["per_case"].items():
        h = exp[n]["hypothesis"]; changed = exp[n]["tip_differs_from_merge"]
        ok_tip = p["tip"]["strict"] == "MATCH"; ok_merge = p["merge"]["strict"] == "MATCH"
        H[h]["n"] += 1; H[h]["tip_match"] += ok_tip; H[h]["merge_match"] += ok_merge
        H[h]["lost"] += ok_merge and not ok_tip; H[h]["gained"] += ok_tip and not ok_merge
        H[h]["changed"] += changed
        if ok_merge and not ok_tip: fails[h].append(n)
    out["hypotheses"] = {h: dict(c) for h, c in H.items()}; out["hypothesis_lost"] = fails
json.dump(out, open(sys.argv[2], "w"), indent=1)
print(json.dumps(out["trees"])); [print(k, v) for k, v in out["deltas"].items() if k.endswith("_counts")]
if exp: print(json.dumps(out["hypotheses"])); print(dict(out["hypothesis_lost"]))
