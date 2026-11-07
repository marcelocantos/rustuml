"""Checker round-6 (Opus) summary: review-6 summarizer with tip 7e13632cd against c25ccbf3c, 9ed52f435, 9fc567332 and cohorts DL, DK, DJ, DH, and one DECLARED validity overlay (OVERLAY=1): a Java reference with exit 100 or 50, or a "Diagram not supported by this release" page, is Java-invalid even when prepare.py found no error marker. Without OVERLAY the rows are classified exactly as round 5 did. Round-4 summary (round-3 summarizer; builds, cache, cohorts, and the T14.63 welcome-match overlay). Outcomes use the unchanged classify.outcome plus the validity adjustment (a case the
manifest marks Java-invalid by raw SVG + stderr is nonpositive). Adds an explicit Welcome classification, which the
unchanged outcome folds into nonpositive: a Java page is a Welcome page when Java exited 0 (fresh cohorts) and its SVG
contains 'Welcome to PlantUML' and no error text ('Syntax Error', 'Empty description', '[From'); a Rust page is a
Welcome page when its SVG contains 'Welcome to RustUML' and Rust exited 0.
Usage: python3 -I summarize_r3.py OUT_JSON"""
import collections, hashlib, json, os, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-opus-tim-r6-recheck"
REPO = Path("/Users/marcelo/.t3/worktrees/rustuml/t3-dfbdde11")
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa: E402
AFTER = "7e13632cd"; BEFORES0 = ["c25ccbf3c", "9ed52f435", "9fc567332"]
FRESH = [c for c in ["dl", "dk", "dj", "dh"] if (V / f"fresh-{c}-manifest.json").exists()] + ["df", "dd", "cc", "cb", "ca", "bz", "by", "bw", "bu", "bd", "ba", "ay", "az", "ax", "av", "at"]
ERR = (b"Syntax Error", b"Empty description", b"[From")
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
    return row["results"]["svg"]["exit"] == 0 and f.exists() and b"Welcome to RustUML" in f.read_bytes()
out = {}
for co in FRESH + ["fresh", "retained", "golden"]:
    BEFORES = [b for b in BEFORES0 if (V / f"runs/{co}-{b}/results.json").exists()]; BUILDS = [AFTER] + BEFORES
    rows = {b: json.loads((V / f"runs/{co}-{b}/results.json").read_text()) for b in BUILDS}
    man = {}
    mf = V / (f"fresh-{co}-manifest.json" if co in FRESH else "fresh-manifest.json")
    if co in FRESH or co == "fresh": man = {m["source"]: m for m in json.loads(mf.read_text())}
    base_dir = V / f"fresh-{co}" if co in FRESH else (V / "fresh" if co == "fresh" else REPO)
    names = lambda r: man.get(r["source"], {}).get("name", r["source"])
    def valid(r):
        if r["source"] not in man: return None
        m = man[r["source"]]
        if os.environ.get("OVERLAY") != "1": return m["observed_valid"]  # round-5 behaviour, verbatim
        ok = m["observed_valid"] in (True, "True")
        if ok:
            if str(m["java_exit"]) in ("100", "50") or b"not supported by this release" in (base_dir / r["golden"]).read_bytes(): ok = False
        return ok
    info = dict(n=len(rows[AFTER]["rows"]), binary_sha256={b: rows[b]["binary_sha256"] for b in BUILDS}, compare_sha256=rows[AFTER]["compare_sha256"])
    if man: info["java_valid"] = sum(valid({"source": k, "golden": m["golden"]}) in (True, "True") for k, m in man.items())
    info["outcomes"] = {b: dict(collections.Counter(adj(r, valid(r)) for r in rows[b]["rows"])) for b in BUILDS}
    info["panics"] = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["panic"])) for b in BUILDS}
    info["timeouts"] = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["timeout"])) for b in BUILDS}
    def jw(r):
        svg = (base_dir / r["golden"]).read_bytes()
        ok_exit = str(man[r["source"]]["java_exit"]) == "0" if r["source"] in man else True
        return ok_exit and b"Welcome to PlantUML" in svg and not any(e in svg for e in ERR)
    java_welcome = {r["source"] for r in rows[AFTER]["rows"] if jw(r)}
    info["java_welcome"] = sorted(names({"source": s}) for s in java_welcome)
    info["rust_welcome"] = {b: sorted(names(r) for r in rows[b]["rows"] if rust_welcome(co, b, r)) for b in BUILDS}
    info["java_welcome_rust_not"] = {b: sorted(names(r) for r in rows[b]["rows"] if r["source"] in java_welcome and not rust_welcome(co, b, r)) for b in BUILDS}
    info["rust_welcome_java_not"] = {b: sorted(names(r) for r in rows[b]["rows"] if r["source"] not in java_welcome and rust_welcome(co, b, r)) for b in BUILDS}
    def adj2(co, b, r):
        if r["source"] in java_welcome and rust_welcome(co, b, r): return "welcome-match"
        return adj(r, valid(r))
    info["outcomes_t1463"] = {b: dict(collections.Counter(adj2(co, b, r) for r in rows[b]["rows"])) for b in BUILDS}
    for before in BEFORES:
        d2 = collections.defaultdict(list)
        for rb, ra in zip(rows[before]["rows"], rows[AFTER]["rows"]):
            ob, oa = adj2(co, before, rb), adj2(co, AFTER, ra)
            if ob == oa: continue
            if ob == "welcome-match": d2["lost_welcome_match"].append(names(rb))
            elif oa == "welcome-match": d2["welcome_match_gain"].append(names(rb))
            else: d2[kind(ob, oa)].append(names(rb))
        info[f"t1463_vs_{before}"] = dict(d2)
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
            if wb != wa:
                d[("welcome_gained" if wa else "welcome_lost") + ("_java_welcome" if ra["source"] in java_welcome else "_java_not_welcome")].append(names(rb))
        info[f"vs_{before}"] = dict(d, svg_byte_changes=len(svgc), svg_changed=svgc if len(svgc) <= 80 else svgc[:80] + ["..."], model_changes=dict(model))
    out[co] = info
Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
for co, info in out.items():
    print(co, info["n"], "valid", info.get("java_valid"), "tip", info["outcomes"][AFTER], "java_welcome", len(info["java_welcome"]),
          "jw_rust_not", len(info["java_welcome_rust_not"][AFTER]), "rw_java_not", len(info["rust_welcome_java_not"][AFTER]))
    for b in [k[3:] for k in info if k.startswith("vs_")]:
        print("   vs", b, {k: (v if isinstance(v, int) else len(v)) for k, v in info[f"vs_{b}"].items() if k not in ("svg_changed",)})
        print("   t1463 vs", b, {k: v for k, v in info[f"t1463_vs_{b}"].items()})
