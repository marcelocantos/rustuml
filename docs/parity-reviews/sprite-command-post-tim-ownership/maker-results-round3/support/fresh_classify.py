"""Classify fresh before/after rows per cohort with Java validity from raw SVG + stderr markers and the comparator verdict."""
import collections, json, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from classify import outcome
man = {m["source"]: m for m in json.loads(Path(sys.argv[1]).read_text())}
before = json.loads(Path(sys.argv[2]).read_text())["rows"]; after = json.loads(Path(sys.argv[3]).read_text())["rows"]
tab = collections.defaultdict(collections.Counter); changes = []
disagree = []
for b, a in zip(before, after):
    assert b["source"] == a["source"]
    m = man[b["source"]]; ob, oa = outcome(b), outcome(a)
    cmp_nonpos = b["comparison"]["exit"] == 2
    if cmp_nonpos == m["observed_valid"]: disagree.append((m["name"], m["observed_valid"], b["comparison"]["exit"]))
    tab[m["cohort"]][("B", ob)] += 1; tab[m["cohort"]][("A", oa)] += 1
    svgc = b["results"]["svg"]["stdout_sha256"] != a["results"]["svg"]["stdout_sha256"]
    if ob != oa or svgc:
        changes.append(dict(name=m["name"], cohort=m["cohort"], java_valid=m["observed_valid"], intended_valid=m["intended_valid"], before=ob, after=oa, svg_changed=svgc,
            after_stderr=a["results"]["svg"]["stderr"][-300:]))
for c, t in sorted(tab.items()):
    print(c, "before", {k[1]: v for k, v in t.items() if k[0] == "B"}, "after", {k[1]: v for k, v in t.items() if k[0] == "A"})
print("marker/comparator disagreements", disagree)
kinds = collections.Counter((c["cohort"], c["before"], c["after"]) for c in changes)
for k, v in sorted(kinds.items()): print("  ", k, v)
bad = [c for c in changes if (c["before"] == "match" and c["after"] != "match") or (c["after"] in ("valid-error", "panic", "timeout") and c["before"] != c["after"]) or (c["before"] == "nonpositive-rejected" and c["after"] != "nonpositive-rejected")]
print("LOSSES", json.dumps(bad, indent=1))
if len(sys.argv) > 4: Path(sys.argv[4]).write_text(json.dumps(dict(changes=changes, losses=bad, disagreements=disagree), indent=1))
