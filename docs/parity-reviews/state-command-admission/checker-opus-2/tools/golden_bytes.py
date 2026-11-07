import hashlib, json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
G = "/Users/marcelo/.t3/worktrees/rustuml/t3-7b3a7e15/test-diagrams/golden"
trees = dict(a.split("=", 1) for a in sys.argv[2:])
files = sorted(os.path.join(r, f) for r, _, fs in os.walk(G) for f in fs if f.endswith(".puml"))
def run(path):
    src = open(path, "rb").read(); out = {}
    for t, b in trees.items():
        try:
            p = subprocess.run([b, "-pipe", "-tsvg"], input=src, capture_output=True, timeout=20)
            out[t] = (p.returncode, hashlib.sha256(p.stdout).hexdigest()[:16])
        except subprocess.TimeoutExpired:
            out[t] = (124, "timeout")
    return os.path.relpath(path, G), out
with ThreadPoolExecutor(12) as ex:
    res = dict(ex.map(run, files))
json.dump(res, open(sys.argv[1], "w"))
print("total", len(res))
for t in trees: print(t, "timeouts", sum(v[t][0] == 124 for v in res.values()), "panics", sum(v[t][0] == 101 for v in res.values()))
for a, b in [("base", "tip"), ("pr", "tip")]:
    d = [k for k, v in res.items() if v[a] != v[b]]
    print(f"{a} vs {b}: identical {len(res)-len(d)}, differ {len(d)}", d[:20])
