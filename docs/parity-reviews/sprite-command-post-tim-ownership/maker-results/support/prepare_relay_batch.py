"""Materialise a relayed Java reference batch into an existing private replay cwd, verifying every hash.

Usage: python3 -I prepare_relay_batch.py MANIFEST RESULTS COHORT DEST
Appends rows to DEST-manifest.json (same row shape as prepare_fresh.py)."""
import base64, hashlib, json, sys
from pathlib import Path
manifest_path, results_path, cohort, dest = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4])
sha = lambda b: hashlib.sha256(b).hexdigest()
man = json.loads(manifest_path.read_text()); res = json.loads(results_path.read_text())
by = {r["name"]: r for r in res["results"]}
for r in res.get("resources", []):
    data = base64.b64decode(r["base64"]); assert sha(data) == r["sha256"]
    (dest / r["path"]).write_bytes(data)
mpath = dest.parent / (dest.name + "-manifest.json")
rows = json.loads(mpath.read_text())
names = {row["name"] for row in rows}
for case in man["cases"]:
    assert case["name"] not in names, case["name"]
    src = Path(case["source"]).read_bytes(); assert sha(src) == case["source_sha256"], case["name"]
    r = by[case["name"]]; assert r["source_sha256"] == case["source_sha256"] and r["stdin_sha256"] == case["source_sha256"]
    svg = base64.b64decode(r["stdout_base64"]); assert sha(svg) == r["stdout_sha256"]
    err = base64.b64decode(r["stderr_base64"]); assert sha(err) == r["stderr_sha256"]
    assert not r["timed_out"]
    (dest / f"{case['name']}.puml").write_bytes(src); (dest / f"{case['name']}.svg").write_bytes(svg)
    rows.append(dict(source=f"{case['name']}.puml", golden=f"{case['name']}.svg", cohort=cohort,
        name=case["name"], intended_valid=case["intended_valid"], axes=case["axes"], mechanism=case.get("mechanism"),
        java_exit=r["exit_code"], java_stderr=err.decode(errors="replace"), java_svg_sha256=r["stdout_sha256"]))
mpath.write_text(json.dumps(rows, indent=1))
print(cohort, len(man["cases"]), "total", len(rows))
