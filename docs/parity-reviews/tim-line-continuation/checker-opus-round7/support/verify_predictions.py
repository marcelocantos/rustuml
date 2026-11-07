"""Re-render a pre-registered prediction file with this checker's builds. Usage: verify_predictions.py PRED MANIFEST MAP(json build->binary) CWD"""
import hashlib, json, os, subprocess, sys
from pathlib import Path
pred = json.loads(Path(sys.argv[1]).read_text()); man = json.loads(Path(sys.argv[2]).read_text()); bmap = json.loads(sys.argv[3]); cwd = sys.argv[4]
src = {c["name"]: c["source"] for c in man["cases"]}
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100", PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
ok = bad = 0
for c in pred["cases"]:
    body = Path(src[c["name"]]).read_bytes()
    for b, exp in c["rust"].items():
        p = subprocess.run([bmap[b], "-tsvg", "-pipe"], input=body, capture_output=True, cwd=cwd, env=env, timeout=60)
        got = dict(exit=p.returncode, svg_sha256=hashlib.sha256(p.stdout).hexdigest(), stderr=p.stderr.decode(errors="replace"))
        if all(got[k] == exp[k] for k in ("exit", "svg_sha256", "stderr")): ok += 1
        else: bad += 1; print("DIFF", c["name"], b, exp["exit"], got["exit"], exp["stderr"][:100], "|", got["stderr"][:100])
print("identical", ok, "different", bad)
