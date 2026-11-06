"""Per-case outcome table for one fresh cohort across the five builds (validity-adjusted as summarize.py), with
W/w marking a Rust Welcome page and J a Java Welcome page.
Usage: python3 -I percase.py COHORT"""
import hashlib, json, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round3-claude-maker"
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa
co = sys.argv[1]; B = ["9834a7194", "93bf53e18", "b0fb43c3e", "a5720e095", "d28dd64a9"]
man = {m["source"]: m for m in json.loads((V / f"fresh-{co}-manifest.json").read_text())}
rows = {b: json.loads((V / f"runs/{co}-{b}/results.json").read_text())["rows"] for b in B}
def adj(row, valid):
    o = outcome(row)
    if valid is False and not o.startswith("nonpositive") and o not in ("panic", "timeout"):
        return "nonpositive-rejected" if row["results"]["svg"]["exit"] != 0 else "nonpositive-accepted"
    return o
def welcome(b, r):
    f = V / f"runs/{co}-{b}/{hashlib.sha256(r['source'].encode()).hexdigest()[:24]}.svg"
    return f.exists() and b"Welcome to RustUML" in f.read_bytes()
print("columns:", " ".join(B))
for i, r in enumerate(rows[B[0]]):
    m = man[r["source"]]
    jw = b"Welcome to PlantUML" in (V / f"fresh-{co}" / m["golden"]).read_bytes()
    outs = [adj(rows[b][i], m["observed_valid"])[:13] + ("+W" if welcome(b, rows[b][i]) else "") for b in B]
    print(f"{m['name']:48s} jv={int(m['observed_valid'])} je={m['java_exit']:>3}{' J' if jw else '  '} " + " ".join(f"{o:15s}" for o in outs))
