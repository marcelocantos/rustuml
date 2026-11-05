"""Side-by-side text/data-source-line extraction: Java SVG vs replayed Rust SVGs for named cases.
Usage: python3 -I inspect.py COHORT NAME [NAME...]   (cohort dir fresh-<c>, runs/<c>-<build>)"""
import hashlib, json, re, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-claude-checker-v2"
co = sys.argv[1]; names = sys.argv[2:]
man = {r["name"]: r for r in json.loads((V / f"fresh-{co}-manifest.json").read_text())}
BUILDS = ["93bf53e18", "9834a7194", "2a9373aca", "594c2b3c5"]
runs = {b: {r["name"]: r for r in json.loads((V / f"runs/{co}-{b}/results.json").read_text())["rows"]} for b in BUILDS}
def summ(svg):
    t = re.findall(r">([^<>]+)</text>", svg); l = re.findall(r'data-source-line="(\d+)"', svg)
    return f"lines={l[:10]} texts={t[:10]}"
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa
for n in names or sorted(man):
    m = man[n]; print("==", n, "java_exit", m["java_exit"], "valid", m["observed_valid"], "intended", m["intended_valid"])
    print("   src:", (V / f"fresh-{co}" / m["source"]).read_text()[:300].replace("\n", "⏎"))
    print("   java:", summ((V / f"fresh-{co}" / m["golden"]).read_text(errors="replace")))
    for b in BUILDS:
        r = runs[b][n]; k = hashlib.sha256(r["source"].encode()).hexdigest()[:24]
        svg = (V / f"runs/{co}-{b}/{k}.svg").read_text(errors="replace")
        print(f"   {b} {outcome(r):22s} exit={r['results']['svg']['exit']} {summ(svg)} {r['results']['svg']['stderr'][-150:].strip()!r}")
