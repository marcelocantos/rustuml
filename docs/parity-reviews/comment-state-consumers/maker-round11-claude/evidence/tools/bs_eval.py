"""Evaluate BS (M10A) against the maker's pre-registered expectations. Usage: python3 -I bs_eval.py <batch-BS.json> <maker-expectations.json> <out.json>"""
import json, sys, collections, pathlib, subprocess, tempfile
OUT = pathlib.Path(sys.argv[1]).parent / "out-BS"
CMPBIN = str(pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round11-claude-maker/cmp/target/release/checker-cmp")
def CMP2(a, b):
    with tempfile.NamedTemporaryFile("w", suffix=".lst", delete=False) as f: f.write(f"{a}\t{b}\n")
    return subprocess.run([CMPBIN, f.name], capture_output=True, text=True).stdout.split("\t")[1] == "MATCH"
rows = json.load(open(sys.argv[1])); exp = json.load(open(sys.argv[2]))["cases"]
M = lambda n, t: rows[n][t + "_strict"] == "MATCH" and rows[n].get(t + "_comments_exact", True)
out = {"by_category": {}, "violations": [], "transitions": collections.Counter(), "losses": {}, "gains": {}}
for e in exp:
    n = e["name"]; cat = e["expect"]; d = out["by_category"].setdefault(cat, collections.Counter())
    tip, mer, base = M(n, "tip"), M(n, "merge"), M(n, "base")
    d["n"] += 1; d["tip_match"] += tip; d["merge_match"] += mer; d["base_match"] += base
    out["transitions"][f"{int(base)}{int(mer)}{int(tip)}"] += 1
    if mer and not tip: out["losses"].setdefault("merge->tip", []).append(n)
    if base and not tip: out["losses"].setdefault("base->tip", []).append(n)
    if tip and not mer: out["gains"].setdefault("merge->tip", []).append(n)
    if cat == "match-iff-control-matches":
        cs = [c for c in e["controls"] if c in rows]
        if len(cs) != len(e["controls"]):  # '(style block: none)': use the unstyled same-family control
            k, f = n.split("_")[2:4]; cs = [f"m10a_{k}_{f}_p"]; d["no_named_control_mapped"] += 1
        ctl = all(M(c, "tip") for c in cs)
        if ctl and not tip: out["violations"].append({"name": n, "kind": "controls match, probe fails", "controls": e["controls"], "ndiff": rows[n]["tip_ndiff"], "head": rows[n].get("tip_diff_head", "")[:300]})
        if not ctl: d["controls_fail"] += 1
        if tip and not ctl: d["probe_match_controls_fail"] += 1
    elif cat == "fail":
        if tip: d["unexpected_match"] += 1; out["violations"].append({"name": n, "kind": "expected fail, matches"})
    elif cat == "same-as-plain":
        twin = e.get("twin") or e.get("controls", [None])[0]
        d["twin_named"] += bool(twin)
        if twin:
            same = CMP2(OUT / f"{twin}.tip.svg", OUT / f"{n}.tip.svg"); d["rust_equiv_twin"] += same
            jsame = CMP2(OUT / f"{twin}.java.svg", OUT / f"{n}.java.svg"); d["java_equiv_twin"] += jsame
            if not same: out["violations"].append({"name": n, "kind": "rust tip differs from plain twin", "twin": twin})
            if not jsame: out["violations"].append({"name": n, "kind": "java differs from plain twin", "twin": twin})
            if M(twin, "tip") != tip: out["violations"].append({"name": n, "kind": "match differs from twin", "twin": twin, "tip": tip})
    elif cat == "control":
        pass
out["by_category"] = {k: dict(v) for k, v in out["by_category"].items()}; out["transitions"] = dict(out["transitions"])
for r in rows.values():
    for t in ("base", "merge", "tip"):
        if r[t + "_exit"] != 0 or r[t + "_panic"]: out.setdefault("nonzero", []).append((r["name"], t, r[t + "_exit"]))
json.dump(out, open(sys.argv[3], "w"), indent=1)
print(json.dumps({k: v for k, v in out.items() if k != "violations"}, indent=1)[:3000]); print("violations", len(out["violations"]))
for v in out["violations"]: print(v)
