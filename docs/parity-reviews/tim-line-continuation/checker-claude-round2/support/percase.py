"""Per-case outcome table for one fresh cohort across the four builds (validity-adjusted as summarize.py).
Usage: python3 -I percase.py COHORT"""
import json, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round2-claude-checker"
sys.path.insert(0, str(V / "support")); from summarize_lib import adj  # noqa
co = sys.argv[1]; B = ["9834a7194", "93bf53e18", "b0fb43c3e", "f2d12d0ef"]
man = {m["source"]: m for m in json.loads((V / f"fresh-{co}-manifest.json").read_text())}
rows = {b: json.loads((V / f"runs/{co}-{b}/results.json").read_text())["rows"] for b in B}
for i, r in enumerate(rows[B[0]]):
    m = man[r["source"]]
    outs = [adj(rows[b][i], m["observed_valid"]) for b in B]
    print(f"{m['name']:50s} jv={int(m['observed_valid'])} je={m['java_exit']:>3} " + " ".join(f"{o[:14]:14s}" for o in outs))
