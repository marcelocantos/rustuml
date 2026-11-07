"""Byte-compare stdout and exit status of two rustuml binaries over a source list."""
import hashlib, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
tip, base, listfile, fmt, out = sys.argv[1:6]
srcs = Path(listfile).read_text().splitlines()
env = {k: v for k, v in os.environ.items() if not k.startswith("RUSTUML")}
def run(b, s):
    p = Path(s)
    try:
        r = subprocess.run([b, fmt, p.name], cwd=p.parent, capture_output=True, timeout=60, env=env)
        return r.returncode, hashlib.sha256(r.stdout).hexdigest()
    except subprocess.TimeoutExpired:
        return "timeout", ""
def one(s):
    return s, run(tip, s), run(base, s)
mism = []
with ThreadPoolExecutor(max_workers=12) as ex:
    for s, a, b in ex.map(one, srcs):
        if a != b: mism.append(f"{s}\t{a}\t{b}")
Path(out).write_text("\n".join(mism) + ("\n" if mism else ""))
print(f"{fmt}: {len(srcs)} sources, {len(mism)} mismatches")
