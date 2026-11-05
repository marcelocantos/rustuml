"""Rust-only run of a staged probe batch (no Java): exit code, stderr tail and SVG text per build.

Usage: python3 -I probe_rust.py BATCH_DIR OUT_JSON BIN [BIN ...]   (cwd = BATCH_DIR/inputs, -tsvg -pipe)
"""
import hashlib, json, os, re, subprocess, sys
from pathlib import Path
batch, out, bins = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100",
           PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
rows = []
for case in json.loads((batch / "reference-manifest.json").read_text())["cases"]:
    body = Path(case["source"]).read_bytes(); row = dict(name=case["name"], intended_valid=case["intended_valid"], results={})
    for b in bins:
        p = subprocess.run([b, "-tsvg", "-pipe"], input=body, capture_output=True, cwd=batch / "inputs", env=env, timeout=60)
        texts = re.findall(r"<text[^>]*>([^<]*)</text>", p.stdout.decode(errors="replace"))
        row["results"][Path(b).name] = dict(exit=p.returncode, stdout_sha256=hashlib.sha256(p.stdout).hexdigest(),
            stderr=p.stderr.decode(errors="replace")[-300:], panic=b"panicked" in p.stderr, texts=texts[:12])
    rows.append(row)
out.write_text(json.dumps(rows, indent=1) + "\n")
for r in rows:
    print(r["name"], {k: v["exit"] for k, v in r["results"].items()}, r["results"][Path(bins[-1]).name]["texts"][-4:])
