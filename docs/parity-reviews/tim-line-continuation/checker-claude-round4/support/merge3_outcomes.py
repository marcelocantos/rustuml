"""Outcome-level merge fidelity: for every case, the merged base M=1f6ffa9f4 against each parent (A=d28dd64a9 round-3
sprite/continuation tip, round-3 runs read-only; B=cb550c0d6 parity-recovery head). Outcome = unchanged classify.outcome
with the validity adjustment and the T14.63 welcome-match overlay (summarize_r4 rules). Reports every case where M is
worse than a parent (lost match / lost rejection / lost welcome match / new valid error / panic) and the parent outcomes.
Usage: python3 -I merge3_outcomes.py OUT_JSON"""
import collections, hashlib, json, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round4-claude-checker"
O3 = Path.home() / "Library/Caches/rustuml-tim-continuation-round3-claude-checker"
REPO = Path("/Users/marcelo/.t3/worktrees/rustuml/t3-1ec8f45b")
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa: E402
FRESH = ["ca", "bz", "by", "bw", "bu", "bd", "ba", "ay", "az", "ax", "av", "at"]
ERR = (b"Syntax Error", b"Empty description", b"[From")
GOOD = {"match": 3, "welcome-match": 3, "nonpositive-rejected": 3}
def run(base, co, b): return {r["source"]: r for r in json.loads((base / f"runs/{co}-{b}/results.json").read_text())["rows"]}
out = {}
for co in FRESH + ["fresh", "retained", "golden"]:
    man = {}
    if co in FRESH: man = {m["source"]: m for m in json.loads((V / f"fresh-{co}-manifest.json").read_text())}
    if co == "fresh": man = {m["source"]: m for m in json.loads((V / "fresh-manifest.json").read_text())}
    gdir = V / f"fresh-{co}" if co in FRESH else (V / "fresh" if co == "fresh" else REPO)
    R = {"A": (O3, "d28dd64a9"), "B": (V, "cb550c0d6"), "M": (V, "1f6ffa9f4")}
    rows = {k: run(base, co, b) for k, (base, b) in R.items()}
    def jw(s, r):
        svg = (gdir / r["golden"]).read_bytes(); ok = str(man[s]["java_exit"]) == "0" if s in man else True
        return ok and b"Welcome to PlantUML" in svg and not any(e in svg for e in ERR)
    def rw(k, r):
        base, b = R[k]; f = base / f"runs/{co}-{b}/{hashlib.sha256(r['source'].encode()).hexdigest()[:24]}.svg"
        return r["results"]["svg"]["exit"] == 0 and f.exists() and b"Welcome to RustUML" in f.read_bytes()
    def oc(k, s):
        r = rows[k][s]
        if jw(s, r) and rw(k, r): return "welcome-match"
        o = outcome(r); v = man[s]["observed_valid"] if s in man else None
        if v is False and not o.startswith("nonpositive") and o not in ("panic", "timeout"):
            o = "nonpositive-rejected" if r["results"]["svg"]["exit"] != 0 else "nonpositive-accepted"
        return o
    worse = []
    for s in rows["M"]:
        om, oa, ob = oc("M", s), oc("A", s), oc("B", s)
        for parent, op in (("A", oa), ("B", ob)):
            if op != om and (op in GOOD and om not in GOOD or om in ("valid-error", "panic", "timeout") and op not in ("valid-error", "panic", "timeout")):
                worse.append(dict(case=man.get(s, {}).get("name", s), vs=parent, parent=op, merged=om, A=oa, B=ob))
    out[co] = worse
Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
for co, w in out.items():
    print(co, len(w))
    for x in w: print("   ", x)
