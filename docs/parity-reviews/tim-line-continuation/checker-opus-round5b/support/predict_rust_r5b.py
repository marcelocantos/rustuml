"""Record Rust outputs for batch R5B at each build before any Java result exists (cwd = a private copy of the probes).
Usage: python3 -I predict_rust_r5b.py PROBE_DIR OUT_JSON BUILD..."""
import hashlib, json, os, shutil, subprocess, sys, tempfile
from pathlib import Path
probe, out, builds = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]
V = Path.home() / "Library/Caches/rustuml-opus-tim-r5-recheck"
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100", PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
cwd = Path(tempfile.mkdtemp(dir=V)); [shutil.copy(p, cwd) for p in probe.iterdir() if p.suffix in (".puml", ".iuml")]
cases = json.loads((probe / "cases.json").read_text()); rows = []
for c in cases:
    src = (probe / f"{c['name']}.puml").read_bytes(); row = dict(name=c["name"], java_prediction_valid=c["intended_valid"], axes=c["axes"], rust={})
    for b in builds:
        p = subprocess.run([str(V / f"bin/rustuml-{b}"), "-tsvg", "-pipe"], input=src, capture_output=True, cwd=cwd, env=env, timeout=60)
        row["rust"][b] = dict(exit=p.returncode, svg_sha256=hashlib.sha256(p.stdout).hexdigest(), stderr=p.stderr.decode(errors="replace")[-600:],
                              welcome=b"Welcome to RustUML" in p.stdout)
    rows.append(row)
shutil.rmtree(cwd)
out.write_text(json.dumps(dict(note="Rust outputs recorded before any Java result for batch R5B; java_prediction_valid is the checker's prediction.",
                               builds=builds, cases=rows), indent=1, ensure_ascii=False))
for r in rows:
    t, b = r["rust"][builds[0]], r["rust"][builds[1]]
    print(f"{r['name']:40} pred={r['java_prediction_valid']!s:5} tip={t['exit']}/{t['svg_sha256'][:8]} base={b['exit']}/{b['svg_sha256'][:8]} " +
          " ".join(f"{x}={r['rust'][x]['exit']}/{r['rust'][x]['svg_sha256'][:8]}" for x in builds[2:]) + ("  ERR " + t["stderr"].strip()[:70] if t["stderr"].strip() else ""))
