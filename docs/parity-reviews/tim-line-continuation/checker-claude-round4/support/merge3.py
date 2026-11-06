"""Three-way merge fidelity of the merged base 1f6ffa9f4 against its parents, per case and mode (svg/ast/ascii/yaml
stdout sha256 and exit code). O = 9834a7194 (code of merge-base adf56ce53; this cache),
A = d28dd64a9 (sprite/continuation side, code of 6ff3ce77e; round-3 checker runs), B = cb550c0d6 (parity-recovery
side, code of f70f107b8 plus help text), M = 1f6ffa9f4 (this cache).
Rule: A==O -> M must equal B (parity-recovery change carried); B==O -> M must equal A (sprite-line change carried);
both differ from O -> conflict, reported with which parent M equals.
Usage: python3 -I merge3.py OUT_JSON"""
import collections, json, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-tim-continuation-round4-claude-checker"
O3 = Path.home() / "Library/Caches/rustuml-tim-continuation-round3-claude-checker"
COH = ["ca", "bz", "by", "bw", "bu", "bd", "ba", "ay", "az", "ax", "av", "at", "fresh", "retained", "golden"]
def load(p):
    return {r["source"]: r for r in json.loads(p.read_text())["rows"]}
def sig(r, m):
    x = r["results"][m]; return (x["exit"], x["stdout_sha256"])
out = {}
for co in COH:
    O = load(V / f"runs/{co}-9834a7194/results.json"); A = load(O3 / f"runs/{co}-d28dd64a9/results.json")
    B = load(V / f"runs/{co}-cb550c0d6/results.json"); M = load(V / f"runs/{co}-1f6ffa9f4/results.json")
    assert O.keys() == A.keys() == B.keys() == M.keys(), co
    c = collections.Counter(); bad = collections.defaultdict(list); conflicts = collections.Counter()
    for s in O:
        for m in ("svg", "ast", "ascii", "yaml"):
            o, a, b, mm = sig(O[s], m), sig(A[s], m), sig(B[s], m), sig(M[s], m)
            if a == o and b == o:
                k = "unchanged" if mm == o else "drift_neither_parent"
            elif a == o:
                k = "pr_carried" if mm == b else "pr_lost"
            elif b == o:
                k = "sprite_carried" if mm == a else "sprite_lost"
            else:
                k = "conflict"; conflicts[(m, "=A" if mm == a else "=B" if mm == b else "neither")] += 1
            c[(m, k)] += 1
            if k in ("pr_lost", "sprite_lost", "drift_neither_parent"): bad[f"{m}:{k}"].append(s)
    out[co] = dict(counts={f"{m}:{k}": v for (m, k), v in sorted(c.items())},
                   conflicts={f"{m}{w}": v for (m, w), v in sorted(conflicts.items())},
                   suspects={k: v[:60] for k, v in bad.items()}, suspect_counts={k: len(v) for k, v in bad.items()})
Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
for co, i in out.items():
    print(co, "suspects", i["suspect_counts"], "conflicts", i["conflicts"])
