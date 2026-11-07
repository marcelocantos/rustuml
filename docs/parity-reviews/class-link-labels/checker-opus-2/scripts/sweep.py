"""Render every golden .puml with base and tip CLIs; record exit + stdout sha; save tip/base svg only when they differ.
usage: sweep.py <golden-root> <outdir> <base-bin> <tip-bin>"""
import hashlib, json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
root, out, base, tip = sys.argv[1:5]
srcs = sorted(os.path.join(d, f) for d, _, fs in os.walk(root) for f in fs if f.endswith(".puml"))
os.makedirs(out, exist_ok=True)
def run(binary, src):
    try:
        p = subprocess.run([binary, "-pipe", "-tsvg"], stdin=open(src, "rb"), capture_output=True, cwd=os.path.dirname(src), timeout=120)
        return p.returncode, p.stdout
    except subprocess.TimeoutExpired:
        return -999, b""
def one(src):
    rel = os.path.relpath(src, root)
    (be, bo), (te, to) = run(base, src), run(tip, src)
    rec = {"base_exit": be, "tip_exit": te, "same": bo == to and be == te}
    if not rec["same"]:
        for lab, data in (("base", bo), ("tip", to)):
            p = os.path.join(out, lab, rel[:-5] + ".svg"); os.makedirs(os.path.dirname(p), exist_ok=True); open(p, "wb").write(data)
    return rel, rec
with ThreadPoolExecutor(12) as ex:
    res = dict(ex.map(one, srcs))
json.dump(res, open(os.path.join(out, "sweep.json"), "w"), indent=0, sort_keys=True)
print(len(res), "goldens;", sum(not r["same"] for r in res.values()), "differ;",
      "tip panics", sum(r["tip_exit"] == 101 for r in res.values()), "base panics", sum(r["base_exit"] == 101 for r in res.values()))
