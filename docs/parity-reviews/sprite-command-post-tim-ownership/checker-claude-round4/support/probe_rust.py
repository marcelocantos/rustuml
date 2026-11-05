"""Rust-only three-build differential over the staged ck4 request (no Java). Each case runs from a private cwd
holding the bundle's resources. Usage: python3 -I probe_rust.py BATCH_DIR OUT_JSON"""
import base64, gzip, hashlib, json, os, subprocess, sys, tempfile
from pathlib import Path
C = Path(os.path.expanduser("~/Library/Caches/rustuml-sprite-round4-claude-checker"))
batch, outp = Path(sys.argv[1]), Path(sys.argv[2])
bundle = json.loads(gzip.open(batch / "reference-input-bundle.json.gz").read())
assert bundle["manifest_sha256"] == hashlib.sha256((batch / "reference-manifest.json").read_bytes()).hexdigest()
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100", PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
cwd = Path(tempfile.mkdtemp(prefix="ck4-cwd-"))
for r in bundle["resources"]: (cwd / r["path"]).write_bytes(base64.b64decode(r["base64"]))
rows = []
for f, case in zip(bundle["files"], bundle["manifest"]["cases"]):
    body = base64.b64decode(f["base64"]); assert hashlib.sha256(body).hexdigest() == case["source_sha256"]
    row = dict(name=case["name"], intended_valid=case["intended_valid"], axes=case["axes"], results={})
    for b in ("93bf53e18", "044f73657", "c9e11493c"):
        p = subprocess.run([str(C / f"bin/rustuml-{b}"), "-tsvg", "-pipe"], input=body, capture_output=True, cwd=cwd, env=env, timeout=60)
        row["results"][b] = dict(exit=p.returncode, svg_sha256=hashlib.sha256(p.stdout).hexdigest(), panic=b"panicked" in p.stderr,
                                 stderr=p.stderr.decode(errors="replace")[:400])
    r = row["results"]; key = lambda b: (r[b]["exit"], r[b]["svg_sha256"])
    row["changed_vs_044f73657"] = key("044f73657") != key("c9e11493c"); row["changed_vs_93bf53e18"] = key("93bf53e18") != key("c9e11493c")
    row["accept_to_error_vs_044f73657"] = r["044f73657"]["exit"] == 0 and r["c9e11493c"]["exit"] != 0
    rows.append(row)
outp.write_text(json.dumps(dict(batch=str(batch), manifest_sha256=bundle["manifest_sha256"], rows=rows), indent=1) + "\n")
print(len(rows), "changed vs 044:", sum(x["changed_vs_044f73657"] for x in rows), "accept->error vs 044:", [x["name"] for x in rows if x["accept_to_error_vs_044f73657"]],
      "error->accept vs 044:", [x["name"] for x in rows if x["results"]["044f73657"]["exit"] != 0 and x["results"]["c9e11493c"]["exit"] == 0], "panics:", sum(v["panic"] for x in rows for v in x["results"].values()))
