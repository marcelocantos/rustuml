"""Checker round-2 summary (adapted from the v2 checker: builds, cache path, cohort list only): outcomes via unchanged classify.outcome, plus a validity-adjusted outcome for fresh cohorts
whose manifest says Java-invalid by raw SVG+stderr (the comparator does not mark 'No valid @start/@end' or crash pages).
Usage: python3 -I summarize.py OUT_JSON"""
import collections, json, sys
if __name__ != "__main__": pass
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round3-claude-checker"
sys.path.insert(0, str(V / "support")); from classify import outcome  # noqa: E402
BUILDS = ["93bf53e18", "9834a7194", "b0fb43c3e", "f2d12d0ef"]; AFTER = "f2d12d0ef"; BEFORES = ["b0fb43c3e", "93bf53e18", "9834a7194"]
FRESH = ["bw", "bu", "bd", "ba", "ay", "az", "ax", "av", "at"]
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
