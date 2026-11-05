"""Materialise staged Java references (main + supplement) into a private replay cwd, verifying every hash."""
import base64, hashlib, json, sys
from pathlib import Path
C, dest = Path(sys.argv[1]), Path(sys.argv[2]); dest.mkdir(exist_ok=False)
sha = lambda b: hashlib.sha256(b).hexdigest()
manifest_rows = []
for sub in ["", "supplement"]:
    base = C / sub
    man = json.loads((base / "reference-manifest.json").read_text())
    res = json.loads((base / "reference-results.json").read_text())
    by = {r["name"]: r for r in res["results"]}
    for r in res.get("resources", []):
        data = base64.b64decode(r["base64"]); assert sha(data) == r["sha256"]
        (dest / r["path"]).write_bytes(data)
    for case in man["cases"]:
        src = Path(case["source"]).read_bytes(); assert sha(src) == case["source_sha256"], case["name"]
        r = by[case["name"]]; assert r["source_sha256"] == case["source_sha256"] and r["stdin_sha256"] == case["source_sha256"]
        svg = base64.b64decode(r["stdout_base64"]); assert sha(svg) == r["stdout_sha256"]
        err = base64.b64decode(r["stderr_base64"]); assert sha(err) == r["stderr_sha256"]
        assert not r["timed_out"]
        (dest / f"{case['name']}.puml").write_bytes(src); (dest / f"{case['name']}.svg").write_bytes(svg)
        manifest_rows.append(dict(source=f"{case['name']}.puml", golden=f"{case['name']}.svg", cohort="fresh-" + (sub or "main"),
            name=case["name"], intended_valid=case["intended_valid"], axes=case["axes"], mechanism=case.get("mechanism"),
            java_exit=r["exit_code"], java_stderr=err.decode(errors="replace"), java_svg_sha256=r["stdout_sha256"]))
(dest.parent / (dest.name + "-manifest.json")).write_text(json.dumps(manifest_rows, indent=1))
print(len(manifest_rows))
