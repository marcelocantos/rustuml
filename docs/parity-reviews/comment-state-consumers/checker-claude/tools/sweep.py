"""Checker sweep: run a RustUML CLI over every X.puml (with sibling X.svg) under roots.

Writes outputs under <outdir>/<relpath>.svg and a status JSON.
Usage: python3 -I sweep.py <bin> <outdir> <base> <root> [<root>...]
"""
import concurrent.futures, hashlib, json, os, pathlib, subprocess, sys

TIMEOUT = 30
WORKERS = 8
DEBUG = "date=1774210426000,tz=AEDT+1100"  # harness GOLDEN_DEBUG pin

def run(binary, outdir, base, src):
    rel = src.relative_to(base)
    env = dict(os.environ, RUSTUML_DEBUG=DEBUG)
    try:
        p = subprocess.run([binary, "-pipe", "-tsvg"], input=src.read_bytes(), capture_output=True,
                           cwd=src.parent, timeout=TIMEOUT, env=env)
        out, err, status = p.stdout, p.stderr, p.returncode
    except subprocess.TimeoutExpired:
        out, err, status = b"", b"", "timeout"
    dst = outdir / rel.with_suffix(".svg")
    dst.parent.mkdir(parents=True, exist_ok=True)
    dst.write_bytes(out)
    panic = b"panicked at" in err
    return str(rel), {"status": status, "panic": panic, "stdout_sha256": hashlib.sha256(out).hexdigest(),
                      "stderr_head": err[:300].decode("utf-8", "replace")}

def main():
    binary, outdir, base, *roots = sys.argv[1:]
    outdir, base = pathlib.Path(outdir), pathlib.Path(base)
    srcs = sorted(p for r in roots for p in pathlib.Path(r).rglob("*.puml") if p.with_suffix(".svg").exists())
    res = {}
    with concurrent.futures.ThreadPoolExecutor(WORKERS) as pool:
        for k, v in pool.map(lambda s: run(binary, outdir, base, s), srcs):
            res[k] = v
    (outdir / "status.json").write_text(json.dumps(res, sort_keys=True, indent=0))
    print(len(res), "sources")

main()
