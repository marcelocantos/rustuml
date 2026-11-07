"""Maker round-5 summary. Outcome = unchanged classify.outcome with the validity adjustment (a case the manifest
marks Java-invalid by raw SVG + stderr is nonpositive) and the T14.63 welcome-match overlay (Java exit 0, SVG has
'Welcome to PlantUML' and no error text; Rust exit 0 and SVG has 'Welcome to RustUML'). Same rules as the round-4
checker's summarize_r4.py and merge3_outcomes.py. For each comparison build B it lists every case where the tip is
WORSE than B (B good = match / welcome-match / nonpositive-rejected and tip not good; or tip valid-error / panic /
timeout and B not), every gain, and byte changes.
Usage: python3 -I summarize_r5.py TIP OUT_JSON BUILD..."""
import collections, hashlib, json, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-opus-tim-r5"
REPO = Path("/Users/marcelo/.t3/worktrees/rustuml/t3-bc3abe07")
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa: E402
TIP, OUT, BUILDS0 = sys.argv[1], sys.argv[2], sys.argv[3:]
FRESH = ["cc", "cb", "ca", "bz", "by", "bw", "bu", "bd", "ba", "ay", "az", "ax", "av", "at"]
ERR = (b"Syntax Error", b"Empty description", b"[From")
GOOD = {"match", "welcome-match", "nonpositive-rejected"}
BAD = {"valid-error", "panic", "timeout"}
out = {}
for co in FRESH + ["fresh", "retained", "golden"]:
    builds = [b for b in BUILDS0 if (V / f"runs/{co}-{b}/results.json").exists()]
    rows = {b: {r["source"]: r for r in json.loads((V / f"runs/{co}-{b}/results.json").read_text())["rows"]} for b in [TIP] + builds}
    man = {}
    if co in FRESH: man = {m["source"]: m for m in json.loads((V / f"fresh-{co}-manifest.json").read_text())}
    if co == "fresh": man = {m["source"]: m for m in json.loads((V / "fresh-manifest.json").read_text())}
    gdir = V / f"fresh-{co}" if co in FRESH else (V / "fresh" if co == "fresh" else REPO)
    name = lambda s: man.get(s, {}).get("name", s)
    def jw(s, r):
        svg = (gdir / r["golden"]).read_bytes(); ok = str(man[s]["java_exit"]) == "0" if s in man else True
        return ok and b"Welcome to PlantUML" in svg and not any(e in svg for e in ERR)
    def rw(b, r):
        f = V / f"runs/{co}-{b}/{hashlib.sha256(r['source'].encode()).hexdigest()[:24]}.svg"
        return r["results"]["svg"]["exit"] == 0 and f.exists() and b"Welcome to RustUML" in f.read_bytes()
    def oc(b, s):
        r = rows[b][s]
        if jw(s, r) and rw(b, r): return "welcome-match"
        o = outcome(r); v = man[s]["observed_valid"] if s in man else None
        if v is False and not o.startswith("nonpositive") and o not in ("panic", "timeout"):
            o = "nonpositive-rejected" if r["results"]["svg"]["exit"] != 0 else "nonpositive-accepted"
        return o
    info = dict(n=len(rows[TIP]), binary_sha256={b: json.loads((V / f"runs/{co}-{b}/results.json").read_text())["binary_sha256"] for b in [TIP] + builds})
    if man: info["java_valid"] = sum(bool(m["observed_valid"]) for m in man.values())
    info["outcomes"] = {b: dict(collections.Counter(oc(b, s) for s in rows[b])) for b in [TIP] + builds}
    info["java_welcome"] = sum(jw(s, r) for s, r in rows[TIP].items())
    info["welcome_match"] = {b: sum(oc(b, s) == "welcome-match" for s in rows[b]) for b in [TIP] + builds}
    info["panics"] = {b: dict(collections.Counter(m for r in rows[b].values() for m, x in r["results"].items() if x["panic"])) for b in [TIP] + builds}
    info["timeouts"] = {b: dict(collections.Counter(m for r in rows[b].values() for m, x in r["results"].items() if x["timeout"])) for b in [TIP] + builds}
    for b in builds:
        worse, gains, other = [], [], []
        svg = 0; model = collections.Counter(); new_panic = []
        for s in rows[TIP]:
            ot, ob = oc(TIP, s), oc(b, s)
            rt, rb = rows[TIP][s], rows[b][s]
            if rt["results"]["svg"]["stdout_sha256"] != rb["results"]["svg"]["stdout_sha256"]: svg += 1
            for m in ("ast", "ascii", "yaml"):
                if rt["results"][m]["stdout_sha256"] != rb["results"][m]["stdout_sha256"]: model[m] += 1
                if rt["results"][m]["panic"] and not rb["results"][m]["panic"]: new_panic.append(f"{name(s)}:{m}")
            if rt["results"]["svg"]["panic"] and not rb["results"]["svg"]["panic"]: new_panic.append(f"{name(s)}:svg")
            if ot == ob: continue
            if (ob in GOOD and ot not in GOOD) or (ot in BAD and ob not in BAD):
                worse.append(dict(case=name(s), build=ob, tip=ot))
            elif ot in GOOD and ob not in GOOD:
                gains.append(dict(case=name(s), build=ob, tip=ot))
            else:
                other.append(dict(case=name(s), build=ob, tip=ot))
        info[f"vs_{b}"] = dict(worse=worse, gains=gains, other=other, new_mode_panics=new_panic, svg_byte_changes=svg, model_changes=dict(model))
    out[co] = info
Path(OUT).write_text(json.dumps(out, indent=1) + "\n")
for co, info in out.items():
    print(co, info["n"], "valid", info.get("java_valid"), "java_welcome", info["java_welcome"], "tip", info["outcomes"][TIP])
    for k, v in info.items():
        if k.startswith("vs_"):
            print(f"   {k}: worse {len(v['worse'])} gains {len(v['gains'])} other {len(v['other'])} new_panics {len(v['new_mode_panics'])} svg {v['svg_byte_changes']} model {v['model_changes']}")
            for w in v["worse"]: print("      WORSE", w)
