"""Build checker round-4 summary.json from replay runs (outcomes via unchanged classify.outcome)."""
import collections, json, os, sys
from pathlib import Path
C = Path(os.path.expanduser("~/Library/Caches/rustuml-sprite-round4-claude-checker"))
sys.path.insert(0, str(C / "support")); from classify import outcome  # noqa: E402
BUILDS = ["93bf53e18", "044f73657", "c9e11493c"]; AFTER = "c9e11493c"
MAN = {"ay": "fresh-ay-manifest.json", "az": "fresh-az-manifest.json", "ax": "fresh-ax-manifest.json", "av": "fresh-av-manifest.json",
       "at": "fresh-at-manifest.json", "fresh": "fresh-manifest.json"}
def kind(b, a, valid):
    if b == a: return None
    if b == "match" and a != "match": return "lost_match"
    if b == "nonpositive-rejected" and a != "nonpositive-rejected": return "lost_rejection"
    if a in ("valid-error", "panic", "timeout"): return "new_" + a.replace("-", "_")
    if a == "match": return "gain"
    if a == "nonpositive-rejected": return "newly_rejected"
    return f"other:{b}->{a}"
out = {}
for co in ["ay", "az", "ax", "av", "at", "fresh", "retained", "golden"]:
    rows = {b: json.loads((C / f"runs/{co}-{b}/results.json").read_text()) for b in BUILDS}
    info = {"n": len(rows[AFTER]["rows"]), "binary_sha256": {b: rows[b]["binary_sha256"] for b in BUILDS},
            "compare_sha256": rows[AFTER]["compare_sha256"]}
    names = {}
    if co in MAN:
        names = {m["source"]: m for m in json.loads((C / MAN[co]).read_text())}
    info["outcomes"] = {b: dict(collections.Counter(outcome(r) for r in rows[b]["rows"])) for b in BUILDS}
    pan = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["panic"])) for b in BUILDS}
    tmo = {b: dict(collections.Counter(m for r in rows[b]["rows"] for m, v in r["results"].items() if v["timeout"])) for b in BUILDS}
    info["panics"], info["timeouts"] = pan, tmo
    for before in ["93bf53e18", "044f73657"]:
        d = collections.defaultdict(list); svgc = 0
        for rb, ra in zip(rows[before]["rows"], rows[AFTER]["rows"]):
            assert rb["source"] == ra["source"]
            if rb["results"]["svg"]["stdout_sha256"] != ra["results"]["svg"]["stdout_sha256"]: svgc += 1
            k = kind(outcome(rb), outcome(ra), None)
            if k: d[k].append(names.get(rb["source"], {}).get("name", rb["source"]))
        info[f"vs_{before}"] = dict({k: (v if k not in ("gain", "newly_rejected") or len(v) <= 40 else len(v)) for k, v in d.items()}, svg_byte_changes=svgc)
    out[co] = info
rej = {}
for f in sorted((C / "rejection").glob("*.json")):
    d = json.loads(f.read_text()); rej[f.stem] = {k: d[k] for k in ("entries", "before_rejections", "after_rejections", "introduced_losses")}
    rej[f.stem]["after_false_accepts"] = d["after_false_accepts"]
out["rejection_contract"] = rej
Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
for co in out:
    if co == "rejection_contract": continue
    print(co, out[co]["n"], {k: (v if isinstance(v, int) else len(v)) for k, v in out[co]["vs_93bf53e18"].items()}, "| vs044", {k: (v if isinstance(v, int) else len(v)) for k, v in out[co]["vs_044f73657"].items()})
