"""Per-cohort before/after results for relay batches, matched by source SHA256.

Usage: python3 -I cohorts.py <repo> <out-before> <out-after> <cmp.tsv> <out.json> <name=manifest.json>...
"""
import hashlib, json, pathlib, re, sys

C = re.compile(r"<!--(.*?)-->", re.S)
repo, ob, oa, cmp_path, out, *batches = sys.argv[1:]
repo = pathlib.Path(repo)
by_sha = {}
for src in (repo / "test-diagrams/perturbations").rglob("*.puml"):
    if src.with_suffix(".svg").exists():
        by_sha.setdefault(hashlib.sha256(src.read_bytes()).hexdigest(), []).append(str(src.relative_to(repo)))
roots = {"before": str(pathlib.Path(ob).resolve()) + "/", "after": str(pathlib.Path(oa).resolve()) + "/"}
cmp = {}
for line in open(cmp_path):
    actual, verdict = line.rstrip("\n").split("\t")[:2]
    for side, root in roots.items():
        if actual.startswith(root):
            cmp[(side, actual[len(root):-4] + ".puml")] = verdict
st = {s: json.load(open(pathlib.Path(d) / "status.json")) for s, d in (("before", ob), ("after", oa))}
report = {}
for spec in batches:
    name, manifest = spec.split("=", 1)
    rows = []
    for case in json.load(open(manifest))["cases"]:
        registered = sorted(by_sha.get(case["source_sha256"], []))
        if not registered:
            rows.append({"case": case["name"], "registered": None})
            continue
        rel = registered[0]
        java = C.findall((repo / rel).with_suffix(".svg").read_text(errors="replace"))
        row = {"case": case["name"], "registered": rel, "intended_valid": case.get("intended_valid")}
        for side, outdir in (("before", ob), ("after", oa)):
            outsvg = (pathlib.Path(outdir) / rel).with_suffix(".svg")
            row[f"{side}_exit"] = st[side].get(rel, {}).get("status")
            row[f"{side}_strict"] = cmp.get((side, rel))
            row[f"{side}_comments_exact"] = outsvg.exists() and C.findall(outsvg.read_text(errors="replace")) == java
        rows.append(row)
    summary = {k: sum(1 for r in rows if r.get(k) in ("MATCH", True)) for k in
               ("before_strict", "after_strict", "before_comments_exact", "after_comments_exact")}
    report[name] = {"cases": len(rows), "registered": sum(1 for r in rows if r["registered"]), **summary, "rows": rows}
    print(name, len(rows), "cases;", summary)
json.dump(report, open(out, "w"), indent=1)
