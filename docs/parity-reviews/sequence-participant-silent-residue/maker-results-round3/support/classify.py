"""Classify before/after replay rows. Validity of fresh references comes from raw Java SVG + stderr, not exit code."""
import collections, json, sys
from pathlib import Path

SYNTAX = ["Syntax Error", "NoSuchElementException", "Welcome to PlantUML", "An error has occured", "kill cannot be used here",
          "swimlane must be defined at the start", "Note already created:", "Parsing syntax error about %", "[From string",
          "No class ", "(Assumed diagram type:", "This feature has been suppressed"]

def outcome(row):
    svg = row["results"]["svg"]; cmp = row["comparison"]["exit"]
    if svg["timeout"]: return "timeout"
    if svg["panic"]: return "panic"
    if cmp == 2: return "nonpositive-rejected" if svg["exit"] != 0 else "nonpositive-accepted"
    if svg["exit"] != 0: return "valid-error"
    return {0: "match", 1: "mismatch"}.get(cmp, f"compare-{cmp}")

def load(p):
    return json.loads(Path(p).read_text())["rows"]

if __name__ == "__main__":
    before, after = load(sys.argv[1]), load(sys.argv[2])
    assert [r["source"] for r in before] == [r["source"] for r in after]
    pairs = collections.Counter(); changes = []
    for b, a in zip(before, after):
        ob, oa = outcome(b), outcome(a); pairs[(ob, oa)] += 1
        if ob != oa or b["results"]["svg"]["stdout_sha256"] != a["results"]["svg"]["stdout_sha256"]:
            changes.append(dict(source=b["source"], cohort=b.get("cohort"), name=b.get("name"), before=ob, after=oa,
                                svg_changed=b["results"]["svg"]["stdout_sha256"] != a["results"]["svg"]["stdout_sha256"]))
    tot = lambda side: collections.Counter(k[side] for k in pairs.elements())
    print("before", dict(tot(0))); print("after", dict(tot(1)))
    for k, v in sorted(pairs.items()):
        if k[0] != k[1]: print("  ", k, v)
    model = collections.Counter()
    for b, a in zip(before, after):
        for m in ("ast", "ascii", "yaml"):
            if b["results"][m]["stdout_sha256"] != a["results"][m]["stdout_sha256"]: model[m] += 1
    print("svg-byte-changes", sum(c["svg_changed"] for c in changes), "model-changes", dict(model))
    if len(sys.argv) > 3: Path(sys.argv[3]).write_text(json.dumps(dict(pairs={f"{k[0]}->{k[1]}": v for k, v in pairs.items()}, changes=changes), indent=1))
