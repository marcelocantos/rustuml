"""Consolidated before/after counts per cohort; lost passes, lost rejections, new valid errors, panics.

Usage: python3 -I summarize.py RUNS BEFORE_SUFFIX AFTER_SUFFIX APPROVED_EXCEPTIONS OUT"""
import collections, json, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from classify import outcome, load
runs, bs, as_, approved_path, out = sys.argv[1:6]
approved = {row["source"] for row in json.loads(Path(approved_path).read_text())}
summary = {}
for cohort in ("retained", "golden", "fresh"):
    before, after = load(f"{runs}/{cohort}-{bs}/results.json"), load(f"{runs}/{cohort}-{as_}/results.json")
    groups = collections.defaultdict(list)
    for b, a in zip(before, after):
        assert b["source"] == a["source"]
        groups[cohort if cohort != "fresh" else b["cohort"]].append((b, a))
    for name, pairs in groups.items():
        ob = collections.Counter(outcome(b) for b, _ in pairs); oa = collections.Counter(outcome(a) for _, a in pairs)
        rows = dict(entries=len(pairs), before=dict(ob), after=dict(oa))
        rows["gains"] = sorted(a["source"] for b, a in pairs if outcome(b) != "match" and outcome(a) == "match")
        rows["lost_matches"] = sorted(a["source"] for b, a in pairs if outcome(b) == "match" and outcome(a) != "match")
        rows["lost_rejections"] = sorted(a["source"] for b, a in pairs if outcome(b) == "nonpositive-rejected" and outcome(a) != "nonpositive-rejected")
        rows["new_rejections"] = sorted(a["source"] for b, a in pairs if outcome(b) == "nonpositive-accepted" and outcome(a) == "nonpositive-rejected")
        rows["new_valid_errors"] = sorted(a["source"] for b, a in pairs if outcome(b) != "valid-error" and outcome(a) == "valid-error")
        rows["after_valid_errors"] = sorted(a["source"] for _, a in pairs if outcome(a) == "valid-error")
        rows["after_valid_errors_outside_approved"] = [s for s in rows["after_valid_errors"] if s not in approved]
        rows["panics"] = {side: sorted({x["source"] for x in (p[i] for p in pairs) for m, r in x["results"].items() if r["panic"]})
                          for i, side in ((0, "before"), (1, "after"))}
        rows["panics_by_mode"] = {side: dict(collections.Counter(m for x in (p[i] for p in pairs) for m, r in x["results"].items() if r["panic"]))
                                  for i, side in ((0, "before"), (1, "after"))}
        rows["timeouts"] = {side: sum(1 for x in (p[i] for p in pairs) for r in x["results"].values() if r["timeout"]) for i, side in ((0, "before"), (1, "after"))}
        rows["svg_byte_changes"] = sum(b["results"]["svg"]["stdout_sha256"] != a["results"]["svg"]["stdout_sha256"] for b, a in pairs)
        summary[name] = rows
Path(out).write_text(json.dumps(summary, indent=1))
for name, r in summary.items():
    print(f"{name:18} n={r['entries']:5} before={r['before']} after={r['after']}")
    print(f"{'':18} gains={len(r['gains'])} lost_matches={len(r['lost_matches'])} lost_rejections={len(r['lost_rejections'])} new_rejections={len(r['new_rejections'])} new_valid_errors={len(r['new_valid_errors'])} after_valid_errors={len(r['after_valid_errors'])} (outside approved {len(r['after_valid_errors_outside_approved'])}) svg_bytes={r['svg_byte_changes']} panics={r['panics_by_mode']} timeouts={r['timeouts']}")
