"""Build the classified manifest for rejection_contract.py: prior checker classification for the 818 staged
references; raw Java SVG + stderr markers (classify.SYNTAX) for relayed batches. Never uses exit codes alone.

Usage: python3 -I classify_manifest.py FRESH_MANIFEST PRIOR_CLASSIFIED REFDIR OUT"""
import json, re, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from classify import SYNTAX
manifest, prior, refdir, out = map(Path, sys.argv[1:5])
prior_by = {r["name"]: r for r in json.loads(prior.read_text())}
rows = []
for row in json.loads(manifest.read_text()):
    if row["name"] in prior_by:
        p = prior_by[row["name"]]; assert p["java_svg_sha256"] == row["java_svg_sha256"]
        rows.append(dict(row, observed_valid=p["observed_valid"], observed_class=p["observed_class"]))
        continue
    svg = (refdir / row["golden"]).read_text(errors="replace")
    markers = [m for m in SYNTAX if m in svg] + (["stderr ERROR"] if row["java_stderr"].startswith("ERROR") else [])
    typed = re.search(r'data-diagram-type="[A-Z]+"', svg) is not None
    valid = typed and not markers
    rows.append(dict(row, observed_valid=valid, observed_class="valid" if valid else "error:" + ",".join(markers or ["untyped"])))
out.write_text(json.dumps(rows, indent=1))
print(len(rows), "valid", sum(r["observed_valid"] for r in rows), "invalid", sum(not r["observed_valid"] for r in rows),
      "intention disagreements", sum(r["observed_valid"] != r["intended_valid"] for r in rows if r["cohort"] == "fresh-batch1"))
