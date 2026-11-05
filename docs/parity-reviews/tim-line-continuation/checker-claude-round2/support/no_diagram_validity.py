"""Parent rule for batch BY: Java exit 50 (ExitStatus.ERROR_50_NO_FILE_FOUND; Pipe#readFirstDiagram returned null, so
no diagram was produced) is INVALID. Applied after strict_validity.py; the earlier verdict is kept as observed_valid_strict.
Usage: python3 -I no_diagram_validity.py MANIFEST PREPARED_DIR"""
import json, sys
from pathlib import Path
man, d = Path(sys.argv[1]), Path(sys.argv[2])
rows = json.loads(man.read_text())
for r in rows:
    r["observed_valid_strict"] = r["observed_valid"]
    if str(r["java_exit"]) == "50":
        assert (d / r["golden"]).read_bytes() == b"", r["name"]
        r["observed_valid"] = False; r["validity_basis"] += "; Java exit 50 with empty stdout: no diagram"
        print("invalid (no diagram):", r["name"])
man.write_text(json.dumps(rows, indent=1))
print(man.name, len(rows), "valid", sum(r["observed_valid"] for r in rows))
