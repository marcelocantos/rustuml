"""Pre-register Rust outcomes for batch MR3 at the five builds, before any Java result exists.
Runs each case from a private copy of the batch inputs (the recorded cwd), -tsvg -pipe, same env as replay.py.
Usage: python3 -I predict_rust.py BATCH_DIR BIN_DIR OUT_JSON"""
import hashlib, json, os, re, shutil, subprocess, sys, tempfile
from pathlib import Path
batch, bindir, outp = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
BUILDS = ["d28dd64a9", "a5720e095", "b0fb43c3e", "93bf53e18", "9834a7194"]
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100",
           PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
sha = lambda b: hashlib.sha256(b).hexdigest()
man = json.loads((batch / "reference-manifest.json").read_text())
cwd = Path(tempfile.mkdtemp()); [shutil.copy(p, cwd) for p in (batch / "inputs").iterdir()]
rows = []
for c in man["cases"]:
    body = Path(c["source"]).read_bytes(); assert sha(body) == c["source_sha256"]
    row = dict(name=c["name"], intended_valid=c["intended_valid"], builds={})
    for b in BUILDS:
        p = subprocess.run([str(bindir / f"rustuml-{b}"), "-tsvg", "-pipe"], input=body, capture_output=True, cwd=cwd, env=env, timeout=60)
        svg = p.stdout.decode(errors="replace")
        texts = [re.sub(r"<[^>]+>", "", t) for t in re.findall(r"<text[^>]*>[^<]*</text>", svg)]
        row["builds"][b] = dict(exit=p.returncode, stdout_sha256=sha(p.stdout), stdout_bytes=len(p.stdout),
            texts=texts[:40], welcome="Welcome to RustUML" in svg, source_lines=sorted(set(int(x) for x in re.findall(r'data-source-line="(\d+)"', svg))),
            stderr=p.stderr.decode(errors="replace")[-400:])
    f, base = row["builds"]["d28dd64a9"], row["builds"]["a5720e095"]
    row["tip_vs_merged_base"] = "identical" if f["stdout_sha256"] == base["stdout_sha256"] and f["exit"] == base["exit"] else "differs"
    rows.append(row)
shutil.rmtree(cwd)
outp.write_text(json.dumps(dict(batch_manifest_sha256=sha((batch / "reference-manifest.json").read_bytes()),
    binaries={b: sha((bindir / f"rustuml-{b}").read_bytes()) for b in BUILDS}, rows=rows), indent=1, ensure_ascii=False))
for r in rows:
    print(f"{r['name']:48s} iv={int(r['intended_valid'])} " + " ".join(f"{b[:4]}:{r['builds'][b]['exit']}" for b in BUILDS) + " W:" + "".join(str(int(r['builds'][b]['welcome'])) for b in BUILDS) + f" {r['tip_vs_merged_base']} tip={'|'.join(r['builds']['d28dd64a9']['texts'][4:8])[:70]}")
