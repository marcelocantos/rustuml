"""Round-2 maker summary (the checker v2 summarize.py with this cache and build labels): outcomes via unchanged classify.outcome, plus a validity-adjusted outcome for fresh cohorts
whose manifest says Java-invalid by raw SVG+stderr (the comparator does not mark 'No valid @start/@end' or crash pages).
Usage: python3 -I summarize.py OUT_JSON"""
import collections, json, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round2-claude-maker"
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa: E402
BUILDS = ["93bf53e18", "9834a7194", "base", "tip"]; AFTER = "tip"; BEFORES = ["base", "93bf53e18", "9834a7194"]
FRESH = ["bu", "bd", "ba", "ay", "az", "ax", "av", "at"]
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
out = {}
for co in FRESH + ["fresh", "retained", "golden"]:
    rows = {b: json.loads((V / f"runs/{co}-{b}/results.json").read_text()) for b in BUILDS}
    man = {}
    mf = V / (f"fresh-{co}-manifest.json" if co in FRESH else "fresh-manifest.json")
    if co in FRESH or co == "fresh": man = {m["source"]: m for m in json.loads(mf.read_text())}
    names = lambda r: man.get(r["source"], {}).get("name", r["source"])
    valid = lambda r: man[r["source"]]["observed_valid"] if r["source"] in man else None
    info = dict(n=len(rows[AFTER]["rows"]), binary_sha256={b: rows[b]["binary_sha256"] for b in BUILDS}, compare_sha256=rows[AFTER]["compare_sha256"])
    if man: info["java_valid"] = sum(m["observed_valid"] for m in man.values())
    info["outcomes"] = {b: dict(collections.Counter(adj(r, valid(r)) for r in rows[b]["rows"])) for b in BUILDS}
    info["panics"] = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["panic"])) for b in BUILDS}
    info["timeouts"] = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["timeout"])) for b in BUILDS}
    for before in BEFORES:
        d = collections.defaultdict(list); svgc = []; model = collections.Counter()
        for rb, ra in zip(rows[before]["rows"], rows[AFTER]["rows"]):
            assert rb["source"] == ra["source"]
            if rb["results"]["svg"]["stdout_sha256"] != ra["results"]["svg"]["stdout_sha256"]: svgc.append(names(rb))
            for m in ("ast", "ascii", "yaml"):
                if rb["results"][m]["stdout_sha256"] != ra["results"][m]["stdout_sha256"]: model[m] += 1
            k = kind(adj(rb, valid(rb)), adj(ra, valid(ra)))
            if k: d[k].append(names(rb))
        info[f"vs_{before}"] = dict(d, svg_byte_changes=len(svgc), svg_changed=svgc if len(svgc) <= 60 else svgc[:60] + ["..."], model_changes=dict(model))
    out[co] = info
Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
for co, info in out.items():
    print(co, info["n"], "valid", info.get("java_valid"), "after", info["outcomes"][AFTER])
    for b in BEFORES:
        print("   vs", b, {k: (v if isinstance(v, int) else len(v)) for k, v in info[f"vs_{b}"].items() if k not in ("svg_changed",)})
