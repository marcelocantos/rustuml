"""Register round-4 checker pairs for the sprite mechanism (T14.6.7 layout).

Usage: python3 -I package_round4.py REPO
Pairs: Java-valid (raw SVG + stderr markers) AY (mk4) and AZ (mk4b) cases, copied byte-identically from the
hash-checked replay dirs into test-diagrams/perturbations/sprite-command-post-tim-ownership/. No case needs a
sibling resource. result = pass iff the after build (c9e11493c) CLI whole-output matches. Java-invalid cases
stay evidence-only.
"""
import hashlib, json, os, shutil, sys
from pathlib import Path
C = Path(os.path.expanduser("~/Library/Caches/rustuml-sprite-round4-claude-checker"))
sys.path.insert(0, str(C / "support"))
from classify import outcome  # noqa: E402
REPO = Path(sys.argv[1]); SP = "sprite-command-post-tim-ownership"; AFTER = "c9e11493c"
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
pairs, evidence_only = [], []
for cohort in ["ay", "az"]:
    manifest = json.loads((C / f"fresh-{cohort}-manifest.json").read_text())
    rows = {b: {r["source"]: r for r in json.loads((C / f"runs/{cohort}-{b}/results.json").read_text())["rows"]}
            for b in ("93bf53e18", "044f73657", AFTER)}
    for case in manifest:
        res = {b: outcome(rows[b][case["source"]]) for b in rows}
        entry = dict(name=case["name"], cohort=cohort.upper(), outcomes=res, axes=case["axes"])
        if not case["observed_valid"]:
            evidence_only.append(dict(entry, java_valid=False, java_markers=case["java_markers"], reason="Java-invalid reference"))
            continue
        dest = REPO / "test-diagrams/perturbations" / SP
        for ext in ("puml", "svg"):
            src = C / f"fresh-{cohort}" / f"{case['name']}.{ext}"; target = dest / f"{case['name']}.{ext}"
            assert not target.exists(), target
            shutil.copyfile(src, target); assert sha(src) == sha(target)
        pairs.append(dict(source=f"test-diagrams/perturbations/{SP}/{case['name']}.puml",
                          golden=f"test-diagrams/perturbations/{SP}/{case['name']}.svg",
                          result="pass" if res[AFTER] == "match" else "fail",
                          axes=case["axes"] + [f"{cohort.upper()} held out", f"after: {res[AFTER]}", f"93bf53e18: {res['93bf53e18']}"]))
out = REPO / "docs/parity-reviews" / SP / "checker-claude-round4"; out.mkdir(parents=True, exist_ok=True)
(out / "registered-pairs.json").write_text(json.dumps(pairs, indent=1) + "\n")
(out / "evidence-only-cases.json").write_text(json.dumps(evidence_only, indent=1) + "\n")
print("pairs", len(pairs), "pass", sum(p["result"] == "pass" for p in pairs), "evidence-only", len(evidence_only))
