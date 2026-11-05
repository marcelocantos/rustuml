"""Run a RustUML CLI over every X.puml with a sibling X.svg, each from its recorded cwd.

Usage: python3 -I sweep.py <bin> <outdir> <repo> <cwd_map.json> <root> [<root>...]
Roots are repository-relative. Output: <outdir>/<rel>.svg, <outdir>/status.json.
"""
import concurrent.futures, hashlib, json, os, pathlib, subprocess, sys

TIMEOUT = 30
WORKERS = 6
DEBUG = "date=1774210426000,tz=AEDT+1100"  # harness GOLDEN_DEBUG pin

def main():
    binary, outdir, repo, cwd_map, *roots = sys.argv[1:]
    outdir, repo = pathlib.Path(outdir), pathlib.Path(repo)
    cwds = json.load(open(cwd_map))
    sources = sorted(p for r in roots for p in (repo / r).rglob("*.puml") if p.with_suffix(".svg").exists())

    def run(src):
        rel = str(src.relative_to(repo))
        cwd = repo / cwds.get(rel, os.path.dirname(rel))
        env = dict(os.environ, RUSTUML_DEBUG=DEBUG)
        try:
            p = subprocess.run([binary, "-pipe", "-tsvg"], input=src.read_bytes(), capture_output=True,
                               cwd=cwd, timeout=TIMEOUT, env=env)
            out, err, status = p.stdout, p.stderr, p.returncode
        except subprocess.TimeoutExpired:
            out, err, status = b"", b"", "timeout"
        dst = outdir / pathlib.Path(rel).with_suffix(".svg")
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_bytes(out)
        return rel, {"status": status, "panic": b"panicked at" in err, "cwd": str(cwd.relative_to(repo)),
                     "stdout_sha256": hashlib.sha256(out).hexdigest(),
                     "stderr_head": err[:300].decode("utf-8", "replace")}

    with concurrent.futures.ThreadPoolExecutor(WORKERS) as pool:
        result = dict(pool.map(run, sources))
    (outdir / "status.json").write_text(json.dumps(result, sort_keys=True, indent=0))
    print(len(result), "sources")

main()
