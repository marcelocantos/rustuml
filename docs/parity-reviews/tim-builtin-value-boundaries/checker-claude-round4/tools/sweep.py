"""Whole-output CLI sweep, each case from its recorded cwd; full stderr kept.
Usage: python3 -I sweep.py <bin> <outdir> <repo> <cwd_map.json> <root>...
Writes <outdir>/<rel>.svg, <outdir>/<rel>.stderr and <outdir>/status.json."""
import concurrent.futures, hashlib, json, os, pathlib, subprocess, sys
TIMEOUT = 30; WORKERS = 10
DEBUG = "date=1774210426000,tz=AEDT+1100"  # harness GOLDEN_DEBUG pin
binary, outdir, repo, cmap, *roots = sys.argv[1:]
outdir, repo = pathlib.Path(outdir), pathlib.Path(repo)
cmap = json.load(open(cmap))
def cwd_for(rel):
    c = cmap.get(rel)
    if c is None or c.startswith("/"):
        return repo / pathlib.Path(rel).parent
    return repo / c
def run(src):
    rel = str(src.relative_to(repo))
    env = dict(os.environ, RUSTUML_DEBUG=DEBUG,
               PLANTUML_STDLIB=str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib"))
    try:
        p = subprocess.run([binary, "-pipe", "-tsvg"], input=src.read_bytes(), capture_output=True,
                           cwd=cwd_for(rel), timeout=TIMEOUT, env=env)
        out, err, status = p.stdout, p.stderr, p.returncode
    except subprocess.TimeoutExpired:
        out, err, status = b"", b"", "timeout"
    dst = outdir / pathlib.Path(rel).with_suffix(".svg")
    dst.parent.mkdir(parents=True, exist_ok=True); dst.write_bytes(out)
    if err: dst.with_suffix(".stderr").write_bytes(err)
    return rel, {"status": status, "panic": b"panicked at" in err, "stdout_sha256": hashlib.sha256(out).hexdigest(),
                 "stderr_sha256": hashlib.sha256(err).hexdigest(),
                 "stderr_head": err[:300].decode("utf-8", "replace"), "cwd": str(cwd_for(rel).relative_to(repo))}
srcs = sorted(p for r in roots for p in (repo / r).rglob("*.puml") if p.with_suffix(".svg").exists())
res = {}
with concurrent.futures.ThreadPoolExecutor(WORKERS) as pool:
    for k, v in pool.map(run, srcs): res[k] = v
(outdir / "status.json").write_text(json.dumps(res, sort_keys=True, indent=0))
print(len(res), "sources")
