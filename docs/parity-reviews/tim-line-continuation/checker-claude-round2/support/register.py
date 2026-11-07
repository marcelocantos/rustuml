"""Register Java-valid BW/BY pairs that need no include/theme resource and are SVG (not ditaa PNG) under
test-diagrams/perturbations/tim-line-continuation/ (parity_perturbations parses with base dir None, T14.6.2), and
write the evidence-only list for the rest. Usage: python3 -I register.py REPO OUTDIR"""
import hashlib, json, sys, shutil
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round2-claude-checker"
repo, out = Path(sys.argv[1]), Path(sys.argv[2])
dest = repo / "test-diagrams/perturbations/tim-line-continuation"; assert dest.is_dir()
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa
BUILDS = ["93bf53e18", "9834a7194", "b0fb43c3e", "f2d12d0ef"]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
reg, ev = [], []
for co, batch in [("bw", "BW"), ("by", "BY")]:
    man = {m["source"]: m for m in json.loads((V / f"fresh-{co}-manifest.json").read_text())}
    runs = {b: {r["source"]: r for r in json.loads((V / f"runs/{co}-{b}/results.json").read_text())["rows"]} for b in BUILDS}
    for s, m in man.items():
        src = V / f"fresh-{co}" / m["source"]; svg = V / f"fresh-{co}" / m["golden"]
        text = src.read_bytes().decode(errors="replace")
        oc = {b: outcome(runs[b][s]) for b in BUILDS}
        rec = dict(name=m["name"], batch=batch, java_valid=m["observed_valid"], intended_valid=m["intended_valid"], java_exit=m["java_exit"],
                   validity_basis=m.get("validity_basis"), axes=m.get("axes"), outcomes_compare_cli=oc, source_sha256=sha(src), java_svg_sha256=sha(svg))
        if not m["observed_valid"]:
            ev.append(dict(rec, reason="Java-invalid reference (raw SVG + stderr, exit 50 = no diagram)")); continue
        if "!include" in text or " from " in text:
            ev.append(dict(rec, reason="needs an include resource beside the source; parity_perturbations parses with base dir None (T14.6.2)")); continue
        if not svg.read_bytes().lstrip().startswith(b"<"):
            ev.append(dict(rec, reason="Java output is a PNG (ditaa); compare_cli cannot compare it (compare-101)")); continue
        for p in (src, svg): assert not (dest / p.name).exists(), p.name
        shutil.copyfile(src, dest / m["source"]); shutil.copyfile(svg, dest / m["golden"])
        reg.append(dict(rec, source=f"test-diagrams/perturbations/tim-line-continuation/{m['source']}",
                        golden=f"test-diagrams/perturbations/tim-line-continuation/{m['golden']}"))
out.mkdir(parents=True, exist_ok=True)
(out / "registered-pairs.json").write_text(json.dumps(reg, indent=1, ensure_ascii=False) + "\n")
(out / "evidence-only-cases.json").write_text(json.dumps(ev, indent=1, ensure_ascii=False) + "\n")
print("registered", len(reg), "evidence-only", len(ev))
for r in reg: print("  ", r["name"], r["outcomes_compare_cli"]["f2d12d0ef"])
