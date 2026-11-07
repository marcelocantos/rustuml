"""Rust-only run of a staged probe batch (no Java): exit, stderr tail, data-source-line values and SVG texts per build.

Usage: python3 -I probe_rust.py BATCH_DIR OUT_JSON BIN [BIN ...]   (cwd = BATCH_DIR/inputs, -tsvg -pipe; --ast for ditaa)
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
        svg = p.stdout.decode(errors="replace")
        res = dict(exit=p.returncode, stdout_sha256=hashlib.sha256(p.stdout).hexdigest(),
            stderr=p.stderr.decode(errors="replace")[-300:], panic=b"panicked" in p.stderr,
            lines=re.findall(r'data-source-line="(\d+)"', svg)[:12], texts=re.findall(r"<text[^>]*>([^<]*)</text>", svg)[:12])
        if "ditaa" in case["name"]:
            a = subprocess.run([b, "--ast", "-pipe"], input=body, capture_output=True, cwd=batch / "inputs", env=env, timeout=60)
            res["ast_rows"] = re.findall(r'^\s+"([^"]*)",$', a.stdout.decode(errors="replace"), re.M)[:12]
            res["ast_head"] = a.stdout.decode(errors="replace")[:200]
        row["results"][Path(b).name] = res
    rows.append(row)
out.write_text(json.dumps(rows, indent=1, ensure_ascii=False) + "\n")
for r in rows:
    t = r["results"][Path(bins[-1]).name]
    print(r["name"], {k: v["exit"] for k, v in r["results"].items()}, t["lines"][-3:], t["texts"][-3:], t.get("ast_rows", "")[:5] if "ast_rows" in t else "", t["stderr"][-90:].strip())
