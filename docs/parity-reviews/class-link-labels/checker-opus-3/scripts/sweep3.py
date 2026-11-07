"""Render every golden .puml with N labelled CLIs (-pipe -tsvg, cwd = the .puml's dir); record exit,
stdout sha256 and the compare_svg_strict verdict against the golden .svg (whole output).
usage: sweep3.py <golden-root> <outdir> <scorer> label=bin ..."""
import hashlib, json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
root, out, scorer, specs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
bins = [s.split("=", 1) for s in specs]
srcs = sorted(os.path.join(d, f) for d, _, fs in os.walk(root) for f in fs if f.endswith(".puml"))
def run(binary, src):
    try:
        p = subprocess.run([binary, "-pipe", "-tsvg"], stdin=open(src, "rb"), capture_output=True, cwd=os.path.dirname(src), timeout=120)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return -999, b"", b"TIMEOUT"
def one(src):
    rel = os.path.relpath(src, root)[:-5]
    rec = {}
    for lab, b in bins:
        code, so, se = run(b, src)
        p = os.path.join(out, lab, rel + ".svg"); os.makedirs(os.path.dirname(p), exist_ok=True); open(p, "wb").write(so)
        rec[lab] = {"exit": code, "sha": hashlib.sha256(so).hexdigest(), "err_sha": hashlib.sha256(se).hexdigest()}
    return rel, rec
with ThreadPoolExecutor(14) as ex:
    res = dict(ex.map(one, srcs))
# strict verdicts
pairs, keys = [], []
for rel in res:
    g = os.path.join(root, rel + ".svg")
    if not os.path.exists(g): continue
    for lab, _ in bins:
        pairs.append(f"{g}\t{os.path.join(out, lab, rel + '.svg')}"); keys.append((rel, lab))
v = subprocess.run([scorer], input="\n".join(pairs) + "\n", capture_output=True, text=True).stdout.splitlines()
assert len(v) == len(keys)
for (rel, lab), s in zip(keys, v):
    res[rel][lab]["strict"] = s
json.dump(res, open(os.path.join(out, "sweep.json"), "w"), indent=0, sort_keys=True)
print(len(res), "goldens")
for lab, _ in bins:
    print(lab, "MATCH", sum(r[lab].get("strict") == "MATCH" for r in res.values()), "panics", sum(r[lab]["exit"] == 101 for r in res.values()), "timeouts", sum(r[lab]["exit"] == -999 for r in res.values()))
