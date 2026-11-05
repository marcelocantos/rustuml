"""Register Java-valid BU/BD pairs that need no include/theme resource under test-diagrams/perturbations/tim-line-continuation/
(parity_perturbations parses with base dir None, so resource-dependent pairs cannot run there; T14.6.2) and write the
evidence-only list for the rest. Usage: python3 -I register.py REPO OUTDIR"""
import hashlib, json, sys, shutil
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-claude-checker-v2"
repo, out = Path(sys.argv[1]), Path(sys.argv[2])
dest = repo / "test-diagrams/perturbations/tim-line-continuation"; dest.mkdir(parents=True, exist_ok=False)
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa
BUILDS = ["93bf53e18", "9834a7194", "2a9373aca", "594c2b3c5"]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
reg, ev = [], []
for co, batch in [("bu", "BU"), ("bd", "BD")]:
    runs = {b: {r["name"]: r for r in json.loads((V / f"runs/{co}-{b}/results.json").read_text())["rows"]} for b in BUILDS}
    for m in json.loads((V / f"fresh-{co}-manifest.json").read_text()):
        src = V / f"fresh-{co}" / m["source"]; svg = V / f"fresh-{co}" / m["golden"]
        text = src.read_text(errors="replace")
        oc = {b: outcome(runs[b][m["name"]]) for b in BUILDS}
        rec = dict(name=m["name"], batch=batch, java_valid=m["observed_valid"], intended_valid=m["intended_valid"], java_exit=m["java_exit"],
                   validity_basis=m.get("validity_basis"), axes=m.get("axes"), outcomes_compare_cli=oc, source_sha256=sha(src), java_svg_sha256=sha(svg))
        needs_resource = "!include" in text or " from ." in text
        if not m["observed_valid"]:
            ev.append(dict(rec, reason="Java-invalid reference (raw SVG + stderr)")); continue
        if needs_resource:
            ev.append(dict(rec, reason="needs an include/theme resource beside the source; parity_perturbations parses with base dir None (T14.6.2)")); continue
        shutil.copyfile(src, dest / m["source"]); shutil.copyfile(svg, dest / m["golden"])
        reg.append(dict(rec, source=f"test-diagrams/perturbations/tim-line-continuation/{m['source']}",
                        golden=f"test-diagrams/perturbations/tim-line-continuation/{m['golden']}"))
out.mkdir(parents=True, exist_ok=True)
(out / "registered-pairs.json").write_text(json.dumps(reg, indent=1) + "\n")
(out / "evidence-only-cases.json").write_text(json.dumps(ev, indent=1) + "\n")
print("registered", len(reg), "evidence-only", len(ev))
