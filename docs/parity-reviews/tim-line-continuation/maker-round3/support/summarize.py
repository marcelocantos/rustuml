"""Maker round-3 summary (the checker round-2 summarize.py with this round's builds and cache path; the outcome
classification is unchanged: classify.outcome plus the validity adjustment for fresh cohorts whose manifest says
Java-invalid by raw SVG + stderr). Adds an explicit Welcome classification, which the unchanged outcome folds into
nonpositive-accepted: Java's page is a Welcome page when its SVG contains 'Welcome to PlantUML', Rust's when its
SVG contains 'Welcome to RustUML'.
Usage: python3 -I summarize.py OUT_JSON"""
import collections, hashlib, json, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round3-claude-maker"
REPO = Path("/Users/marcelo/.t3/worktrees/rustuml/t3-78d2182a")
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa: E402
AFTER = "d28dd64a9"; BEFORES = ["a5720e095", "b0fb43c3e", "93bf53e18", "9834a7194"]; BUILDS = [AFTER] + BEFORES
FRESH = ["by", "bw", "bu", "bd", "ba", "ay", "az", "ax", "av", "at"]
def adj(row, valid):
    o = outcome(row)
    if valid is False and not o.startswith("nonpositive") and o not in ("panic", "timeout"):
        return "nonpositive-rejected" if row["results"]["svg"]["exit"] != 0 else "nonpositive-accepted"
    return o
def kind(b, a):
    if b == a: return None
    if b == "match": return "lost_match"
    if b == "nonpositive-rejected": return "lost_rejection"
    if a in ("valid-error", "panic", "timeout"): return "new_" + a.replace("-", "_")
    if a == "match": return "gain"
    if a == "nonpositive-rejected": return "newly_rejected"
    return f"other:{b}->{a}"
def rust_welcome(co, build, row):
    key = hashlib.sha256(row["source"].encode()).hexdigest()[:24]
    f = V / f"runs/{co}-{build}/{key}.svg"
    return f.exists() and b"Welcome to RustUML" in f.read_bytes()
out = {}
for co in FRESH + ["fresh", "retained", "golden"]:
    rows = {b: json.loads((V / f"runs/{co}-{b}/results.json").read_text()) for b in BUILDS}
    man = {}
    mf = V / (f"fresh-{co}-manifest.json" if co in FRESH else "fresh-manifest.json")
    if co in FRESH or co == "fresh": man = {m["source"]: m for m in json.loads(mf.read_text())}
    base_dir = V / f"fresh-{co}" if co in FRESH else (V / "fresh" if co == "fresh" else REPO)
    names = lambda r: man.get(r["source"], {}).get("name", r["source"])
    valid = lambda r: man[r["source"]]["observed_valid"] if r["source"] in man else None
    info = dict(n=len(rows[AFTER]["rows"]), binary_sha256={b: rows[b]["binary_sha256"] for b in BUILDS}, compare_sha256=rows[AFTER]["compare_sha256"])
    if man: info["java_valid"] = sum(m["observed_valid"] for m in man.values())
    info["outcomes"] = {b: dict(collections.Counter(adj(r, valid(r)) for r in rows[b]["rows"])) for b in BUILDS}
    info["panics"] = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["panic"])) for b in BUILDS}
    info["timeouts"] = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["timeout"])) for b in BUILDS}
    # Welcome classification.
    java_welcome = {r["source"] for r in rows[AFTER]["rows"] if b"Welcome to PlantUML" in (base_dir / r["golden"]).read_bytes()}
    info["java_welcome"] = sorted(names({"source": s}) for s in java_welcome)
    info["rust_welcome"] = {b: sorted(names(r) for r in rows[b]["rows"] if rust_welcome(co, b, r)) for b in BUILDS}
    for before in BEFORES:
        d = collections.defaultdict(list); svgc = []; model = collections.Counter()
        for rb, ra in zip(rows[before]["rows"], rows[AFTER]["rows"]):
            assert rb["source"] == ra["source"]
            if rb["results"]["svg"]["stdout_sha256"] != ra["results"]["svg"]["stdout_sha256"]: svgc.append(names(rb))
            for m in ("ast", "ascii", "yaml"):
                if rb["results"][m]["stdout_sha256"] != ra["results"][m]["stdout_sha256"]: model[m] += 1
            k = kind(adj(rb, valid(rb)), adj(ra, valid(ra)))
            if k: d[k].append(names(rb))
            wb, wa = rust_welcome(co, before, rb), rust_welcome(co, AFTER, ra)
            jw = ra["source"] in java_welcome
            if wb != wa:
                d[("welcome_gained" if wa else "welcome_lost") + ("_java_welcome" if jw else "_java_not_welcome")].append(names(rb))
        info[f"vs_{before}"] = dict(d, svg_byte_changes=len(svgc), svg_changed=svgc if len(svgc) <= 60 else svgc[:60] + ["..."], model_changes=dict(model))
    out[co] = info
Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
for co, info in out.items():
    print(co, info["n"], "valid", info.get("java_valid"), "after", info["outcomes"][AFTER], "java_welcome", len(info["java_welcome"]))
    for b in BEFORES:
        print("   vs", b, {k: (v if isinstance(v, int) else len(v)) for k, v in info[f"vs_{b}"].items() if k not in ("svg_changed",)})
