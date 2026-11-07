"""Run the staged mk4 inputs through the before and after CLIs (no Java, no comparator) and record
per-mode exit codes and stdout hashes, so the Rust-side effect of the port is visible before relay.

Usage: python3 -I mk4_rust_only.py BATCH_DIR BEFORE_BIN AFTER_BIN OUT_JSON
"""
import hashlib, json, os, subprocess, sys
from pathlib import Path
batch, before, after, out = Path(sys.argv[1]), sys.argv[2], sys.argv[3], Path(sys.argv[4])
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100",
           PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
MODES = [("svg", ["-tsvg", "-pipe"]), ("ast", ["--ast", "-pipe"]), ("ascii", ["-ttxt", "-pipe"]), ("yaml", ["--yaml", "-pipe"])]
man = json.loads((batch / "reference-manifest.json").read_text())
rows = []
for case in man["cases"]:
    body = Path(case["source"]).read_bytes()
    row = dict(name=case["name"], intended_valid=case["intended_valid"], axes=case["axes"], results={})
    for label, binary in (("before", before), ("after", after)):
        row["results"][label] = {}
        for mode, flags in MODES:
            p = subprocess.run([binary, *flags], input=body, capture_output=True, cwd=batch / "inputs", env=env, timeout=60)
            row["results"][label][mode] = dict(exit=p.returncode, stdout_sha256=hashlib.sha256(p.stdout).hexdigest(),
                                               panic=b"panicked" in p.stderr, stderr=p.stderr.decode(errors="replace")[-400:])
    b, a = row["results"]["before"]["svg"], row["results"]["after"]["svg"]
    row["svg_changed"] = b["stdout_sha256"] != a["stdout_sha256"]
    row["svg_exit"] = f"{b['exit']}->{a['exit']}"
    rows.append(row)
out.write_text(json.dumps(dict(before=before, after=after, rows=rows), indent=1))
print("cases", len(rows), "svg changed", sum(r["svg_changed"] for r in rows),
      "panics", sum(v["panic"] for r in rows for s in r["results"].values() for v in s.values()))
for r in rows:
    print(f"{r['svg_exit']:6} {'CHANGED' if r['svg_changed'] else 'same   '} {r['name']}")
