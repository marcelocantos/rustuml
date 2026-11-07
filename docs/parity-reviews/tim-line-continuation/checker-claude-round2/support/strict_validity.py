"""Rewrite observed_valid in a prepare.py manifest from raw Java SVG + stderr with the brief's rule:
prepare.py's syntax markers, plus 'No valid @start/@end found' (exit-100 no-block page) and Java crash pages
(exception text in SVG or stderr) are INVALID. The marker-only verdict is kept as observed_valid_markers.
Usage: python3 -I strict_validity.py MANIFEST PREPARED_DIR"""
import json, sys
from pathlib import Path
man, d = Path(sys.argv[1]), Path(sys.argv[2])
EXTRA = ["No valid @start/@end found", "java.lang.", "Exception"]
rows = json.loads(man.read_text())
for r in rows:
    text = (d / r["golden"]).read_bytes().decode(errors="replace") + "\n" + r["java_stderr"]
    hits = [m for m in EXTRA if m in text]
    r["observed_valid_markers"] = r["observed_valid"]
    r["observed_valid"] = r["observed_valid"] and not hits
    r["validity_basis"] = "raw Java SVG + stderr: prepare.py markers + " + (", ".join(hits) if hits else "no extra invalid marker")
    if r["observed_valid"] != r["observed_valid_markers"]: print("invalid by strict rule:", r["name"], r["java_exit"], hits)
man.write_text(json.dumps(rows, indent=1))
print(man.name, len(rows), "valid", sum(r["observed_valid"] for r in rows))
