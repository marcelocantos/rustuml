"""Materialise staged Java references into a private replay cwd, verifying every hash.

Usage: python3 -I prepare.py DEST COHORT=DIR [COHORT=DIR ...]
DIR holds reference-manifest.json and reference-results.json. Resources go into DEST.
Validity is classified from raw Java SVG + stderr markers (not the exit code); the
comparator's own nonpositive verdict (exit 2) is recorded at replay time as well.
"""
import base64, hashlib, json, sys
from pathlib import Path
SYNTAX = ["Syntax Error", "NoSuchElementException", "Welcome to PlantUML", "An error has occured", "Cannot read",
          "No sprite defined", "graylevel are allowed", "[From string", "(Assumed diagram type:", "This feature has been suppressed"]
dest = Path(sys.argv[1]); dest.mkdir(exist_ok=False)
sha = lambda b: hashlib.sha256(b).hexdigest()
rows = []
for arg in sys.argv[2:]:
    cohort, base = arg.split("=", 1); base = Path(base)
    man = json.loads((base / "reference-manifest.json").read_text())
    res = json.loads((base / "reference-results.json").read_text())
    by = {r["name"]: r for r in res["results"]}
    assert len(by) == len(man["cases"]), cohort
    bundle = None
    for r in res.get("resources", []):
        if "base64" not in r:
            import gzip
            bundle = bundle or {x["path"]: x for x in json.loads(gzip.open(base / "reference-input-bundle.json.gz").read())["resources"]}
            r = dict(r, base64=bundle[r["path"]]["base64"])
        data = base64.b64decode(r["base64"]); assert sha(data) == r["sha256"]
        p = dest / r["path"]; assert not p.exists(), r["path"]; p.write_bytes(data)
    for case in man["cases"]:
        src = Path(case["source"]).read_bytes(); assert sha(src) == case["source_sha256"], case["name"]
        r = by[case["name"]]; assert r["source_sha256"] == case["source_sha256"] and r["stdin_sha256"] == case["source_sha256"]
        svg = base64.b64decode(r["stdout_base64"]); assert sha(svg) == r["stdout_sha256"]
        err = base64.b64decode(r["stderr_base64"]); assert sha(err) == r["stderr_sha256"]
        assert not r["timed_out"]
        text = svg.decode(errors="replace") + "\n" + err.decode(errors="replace")
        markers = [m for m in SYNTAX if m in text]
        assert not (dest / f"{case['name']}.puml").exists(), case["name"]
        (dest / f"{case['name']}.puml").write_bytes(src); (dest / f"{case['name']}.svg").write_bytes(svg)
        rows.append(dict(source=f"{case['name']}.puml", golden=f"{case['name']}.svg", cohort=cohort, name=case["name"],
            intended_valid=case["intended_valid"], axes=case.get("axes"), mechanism=case.get("mechanism"),
            java_exit=r["exit_code"], java_stderr=err.decode(errors="replace")[-2000:], java_svg_sha256=r["stdout_sha256"],
            java_markers=markers, observed_valid=not markers))
(dest.parent / (dest.name + "-manifest.json")).write_text(json.dumps(rows, indent=1))
import collections
print(len(rows), collections.Counter((r["cohort"], r["observed_valid"], r["intended_valid"]) for r in rows))
