"""Rust-only run of a probe dir through several builds (svg mode, -pipe, cwd = probe dir).
Usage: python3 -I probe_rust.py PROBE_DIR OUT_JSON BUILD..."""
import hashlib, json, os, re, subprocess, sys
from pathlib import Path
probe, out, builds = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]
B = Path.home() / "Library/Caches/rustuml-tim-continuation-claude-checker/bin"
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100",
           PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
rows = []
for case in json.loads((probe / "cases.json").read_text()):
    body = (probe / f"{case['name']}.puml").read_bytes(); row = dict(case, builds={})
    for b in builds:
        p = subprocess.run([str(B / f"rustuml-{b}"), "-tsvg", "-pipe"], input=body, capture_output=True, cwd=probe, env=env, timeout=60)
        svg = p.stdout.decode(errors="replace")
        texts = re.findall(r">([^<>]+)</text>", svg)
        lines = re.findall(r'data-source-line="(\d+)"', svg)
        row["builds"][b] = dict(exit=p.returncode, sha=hashlib.sha256(p.stdout).hexdigest()[:12], stderr=p.stderr.decode(errors="replace")[-300:],
                                panic=b"panicked" in p.stderr, texts=texts[:12], source_lines=lines[:12])
    rows.append(row)
out.write_text(json.dumps(rows, indent=1))
for r in rows:
    print(r["name"])
    for b, x in r["builds"].items():
        print(f"  {b} exit={x['exit']} {x['sha']} lines={x['source_lines']} texts={x['texts'][:6]} {x['stderr'][-120:].strip()!r}")
