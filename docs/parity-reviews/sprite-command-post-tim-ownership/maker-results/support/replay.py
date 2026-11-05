"""Replay a manifest through one actual rustuml CLI binary (svg/ast/ascii/yaml) and the unchanged comparator.

Usage: python3 -I replay.py LABEL MANIFEST RUSTUML COMPARE_CLI OUTDIR REPO
MANIFEST: JSON array of {source, golden, ...}; relative paths resolve against REPO (also the cwd).
"""
import concurrent.futures, hashlib, json, os, subprocess, sys, time
from pathlib import Path

label, manifest, binary, compare, outdir, repo = sys.argv[1:7]
repo = Path(repo); out = Path(outdir); out.mkdir(parents=True, exist_ok=False)
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100",
           PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
sha = lambda b: hashlib.sha256(b).hexdigest()
MODES = [("svg", ["-tsvg", "-pipe"]), ("ast", ["--ast", "-pipe"]), ("ascii", ["-ttxt", "-pipe"]), ("yaml", ["--yaml", "-pipe"])]

def one(item):
    src = repo / item["source"]; body = src.read_bytes()
    key = sha(item["source"].encode())[:24]
    row = dict(item, source_sha256=sha(body), results={})
    for mode, flags in MODES:
        try:
            p = subprocess.run([binary, *flags], input=body, capture_output=True, cwd=repo, env=env, timeout=60)
            code, so, se, to = p.returncode, p.stdout, p.stderr, False
        except subprocess.TimeoutExpired as e:
            code, so, se, to = None, e.stdout or b"", e.stderr or b"", True
        f = out / f"{key}.{mode}"; f.write_bytes(so)
        row["results"][mode] = dict(exit=code, timeout=to, stdout_sha256=sha(so), stderr=se.decode(errors="replace")[-4000:],
                                    panic=b"panicked" in se)
        if mode == "svg":
            p = subprocess.run([compare, str(repo / item["golden"]), str(f)], capture_output=True, timeout=60)
            row["comparison"] = dict(exit=p.returncode, stdout=p.stdout.decode(errors="replace")[-2000:], stderr=p.stderr.decode(errors="replace")[-2000:])
    return row

t = time.monotonic()
items = json.loads(Path(manifest).read_text())
with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
    rows = list(pool.map(one, items))
(out / "results.json").write_text(json.dumps(dict(label=label, binary=binary, binary_sha256=sha(Path(binary).read_bytes()),
    compare=compare, compare_sha256=sha(Path(compare).read_bytes()), manifest=manifest, rows=rows), indent=1))
print(label, len(rows), round(time.monotonic() - t), flush=True)
