"""Maker round-4 summary (derived from the round-3 checker's summarize_r3.py; classify.outcome unchanged).
Outcomes: classify.outcome plus the validity adjustment (a case the manifest marks Java-invalid by raw SVG + stderr is
nonpositive), plus 'welcome-match' when Java shows its Welcome page and Rust shows its own (T14.63: a match).
A Java page is a Welcome page when Java exited 0 (fresh cohorts) and its SVG contains 'Welcome to PlantUML' and no
error text ('Syntax Error', 'Empty description', '[From'); a Rust page is one when its SVG contains 'Welcome to RustUML'
and Rust exited 0.
Usage: python3 -I summarize_r4.py TIP OUT_JSON"""
import collections, hashlib, json, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round4-claude-maker"
REPO = Path("/Users/marcelo/.t3/worktrees/rustuml/t3-94686399")
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa: E402
AFTER = sys.argv[1]; BEFORES = ["1f6ffa9f4", "93bf53e18", "9834a7194"]; BUILDS = [AFTER] + BEFORES
FRESH = ["ca", "bz", "by", "bw", "bu", "bd", "ba", "ay", "az", "ax", "av", "at"]
ERR = (b"Syntax Error", b"Empty description", b"[From")
MATCHES = ("match", "welcome-match")
def kind(b, a):
    if b == a: return None
    if b in MATCHES: return "lost_match"
    if b == "nonpositive-rejected": return "lost_rejection"
    if a in ("valid-error", "panic", "timeout"): return "new_" + a.replace("-", "_")
    if a in MATCHES: return "gain"
    if a == "nonpositive-rejected": return "newly_rejected"
    return f"other:{b}->{a}"
def rust_welcome(co, build, row):
    key = hashlib.sha256(row["source"].encode()).hexdigest()[:24]
    f = V / f"runs/{co}-{build}/{key}.svg"
    return row["results"]["svg"]["exit"] == 0 and f.exists() and b"Welcome to RustUML" in f.read_bytes()
out = {}
for co in FRESH + ["fresh", "retained", "golden"]:
    rows = {b: json.loads((V / f"runs/{co}-{b}/results.json").read_text()) for b in BUILDS}
    man = {}
    mf = V / (f"fresh-{co}-manifest.json" if co in FRESH else "fresh-manifest.json")
    if co in FRESH or co == "fresh": man = {m["source"]: m for m in json.loads(mf.read_text())}
    base_dir = V / f"fresh-{co}" if co in FRESH else (V / "fresh" if co == "fresh" else REPO)
    names = lambda r: man.get(r["source"], {}).get("name", r["source"])
    valid = lambda r: man[r["source"]]["observed_valid"] if r["source"] in man else None
    def jw(r):
        svg = (base_dir / r["golden"]).read_bytes()
        ok_exit = str(man[r["source"]]["java_exit"]) == "0" if r["source"] in man else True
        return ok_exit and b"Welcome to PlantUML" in svg and not any(e in svg for e in ERR)
    java_welcome = {r["source"] for r in rows[AFTER]["rows"] if jw(r)}
    def oc(build, r):
        if r["source"] in java_welcome and rust_welcome(co, build, r): return "welcome-match"
        o = outcome(r); v = valid(r)
        if v is False and not o.startswith("nonpositive") and o not in ("panic", "timeout"):
            return "nonpositive-rejected" if r["results"]["svg"]["exit"] != 0 else "nonpositive-accepted"
        return o
    info = dict(n=len(rows[AFTER]["rows"]), binary_sha256={b: rows[b]["binary_sha256"] for b in BUILDS}, compare_sha256=rows[AFTER]["compare_sha256"])
    if man: info["java_valid"] = sum(m["observed_valid"] for m in man.values())
    info["outcomes"] = {b: dict(collections.Counter(oc(b, r) for r in rows[b]["rows"])) for b in BUILDS}
    info["panics"] = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["panic"])) for b in BUILDS}
    info["timeouts"] = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["timeout"])) for b in BUILDS}
    info["java_welcome"] = sorted(names({"source": s}) for s in java_welcome)
    info["rust_welcome"] = {b: sorted(names(r) for r in rows[b]["rows"] if rust_welcome(co, b, r)) for b in BUILDS}
    info["java_welcome_rust_not"] = {b: sorted(names(r) for r in rows[b]["rows"] if r["source"] in java_welcome and not rust_welcome(co, b, r)) for b in BUILDS}
    info["rust_welcome_java_not"] = {b: sorted(names(r) for r in rows[b]["rows"] if r["source"] not in java_welcome and rust_welcome(co, b, r)) for b in BUILDS}
    info["per_case_tip"] = {names(r): oc(AFTER, r) for r in rows[AFTER]["rows"]} if co in FRESH else None
    for before in BEFORES:
        d = collections.defaultdict(list); svgc = []; model = collections.Counter(); modelc = collections.defaultdict(list)
        for rb, ra in zip(rows[before]["rows"], rows[AFTER]["rows"]):
            assert rb["source"] == ra["source"]
            if rb["results"]["svg"]["stdout_sha256"] != ra["results"]["svg"]["stdout_sha256"]: svgc.append(names(rb))
            for m in ("ast", "ascii", "yaml"):
                if rb["results"][m]["stdout_sha256"] != ra["results"][m]["stdout_sha256"]: model[m] += 1; modelc[m].append(names(rb))
            k = kind(oc(before, rb), oc(AFTER, ra))
            if k: d[k].append(names(rb))
        info[f"vs_{before}"] = dict(d, svg_byte_changes=len(svgc), svg_changed=svgc if len(svgc) <= 120 else svgc[:120] + ["..."],
                                    model_changes=dict(model), model_changed={m: v[:60] for m, v in modelc.items()})
    out[co] = info
Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")
for co, info in out.items():
    print(co, info["n"], "valid", info.get("java_valid"), "tip", info["outcomes"][AFTER], "java_welcome", len(info["java_welcome"]),
          "jw_rust_not", len(info["java_welcome_rust_not"][AFTER]), "rw_java_not", len(info["rust_welcome_java_not"][AFTER]))
    for b in BEFORES:
        print("   vs", b, {k: (v if isinstance(v, int) else len(v)) for k, v in info[f"vs_{b}"].items() if k not in ("svg_changed", "model_changed")})
