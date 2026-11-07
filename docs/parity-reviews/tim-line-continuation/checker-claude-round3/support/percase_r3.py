"""Per-case outcome table for one fresh cohort at the four builds (validity-adjusted as summarize_r3.py), with Java
Welcome (exit 0, 'Welcome to PlantUML', no error text) and Rust Welcome marked as W.
Usage: python3 -I percase_r3.py COHORT"""
import hashlib, json, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round3-claude-checker"
sys.path.insert(0, str(V / "support")); from summarize_lib import adj  # noqa
co = sys.argv[1]; B = ["9834a7194", "93bf53e18", "a5720e095", "d28dd64a9"]
man = {m["source"]: m for m in json.loads((V / f"fresh-{co}-manifest.json").read_text())}
rows = {b: json.loads((V / f"runs/{co}-{b}/results.json").read_text())["rows"] for b in B}
ERR = (b"Syntax Error", b"Empty description", b"[From")
def rw(b, r):
    f = V / f"runs/{co}-{b}/{hashlib.sha256(r['source'].encode()).hexdigest()[:24]}.svg"
    return r["results"]["svg"]["exit"] == 0 and b"Welcome to RustUML" in f.read_bytes()
print(f"{'case':46s} jv je   " + " ".join(f"{b[:9]:16s}" for b in B))
for i, r in enumerate(rows[B[0]]):
    m = man[r["source"]]; svg = (V / f"fresh-{co}" / m["golden"]).read_bytes()
    jw = str(m["java_exit"]) == "0" and b"Welcome to PlantUML" in svg and not any(e in svg for e in ERR)
    outs = [adj(rows[b][i], m["observed_valid"])[:14] + ("/W" if rw(b, rows[b][i]) else "") for b in B]
    print(f"{m['name'][:46]:46s} {int(m['observed_valid'])}{'W' if jw else ' '} {m['java_exit']:>3} " + " ".join(f"{o:16s}" for o in outs))
