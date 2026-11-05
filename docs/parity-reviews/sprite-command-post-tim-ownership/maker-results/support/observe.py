"""Local CLI observations (no Java verdict) for a staged request: exit and SVG hash before/after.

Usage: python3 -I observe.py MANIFEST CWD BEFORE AFTER OUT"""
import hashlib, json, os, subprocess, sys
from pathlib import Path
manifest, cwd, before, after, out = sys.argv[1:6]
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100",
           PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
sha = lambda b: hashlib.sha256(b).hexdigest()
rows = []
for case in json.loads(Path(manifest).read_text())["cases"]:
    body = Path(case["source"]).read_bytes(); assert sha(body) == case["source_sha256"]
    row = dict(name=case["name"], source_sha256=case["source_sha256"], intended_valid=case["intended_valid"], axes=case["axes"])
    for label, binary in (("before", before), ("after", after)):
        p = subprocess.run([binary, "-tsvg", "-pipe"], input=body, capture_output=True, cwd=cwd, env=env, timeout=60)
        row[label] = dict(exit=p.returncode, svg_sha256=sha(p.stdout), stderr_tail=p.stderr.decode(errors="replace").splitlines()[-3:])
    rows.append(row)
Path(out).write_text(json.dumps(dict(note="local CLI observations only; no Java verdict", cases=rows), indent=1))
for r in rows:
    print(f"{r['name']:45} valid={r['intended_valid']!s:5} before={r['before']['exit']} after={r['after']['exit']}")
