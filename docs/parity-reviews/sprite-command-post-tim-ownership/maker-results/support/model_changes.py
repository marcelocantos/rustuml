"""Explain every AST/ASCII/YAML stdout change between before/after captures of one cohort."""
import collections, hashlib, json, sys
sys.path.insert(0, sys.argv[1] + "/support")
from classify import outcome, load
R, label = sys.argv[1], sys.argv[2]
b, a = load(f"{R}/runs/{label}-before/results.json"), load(f"{R}/runs/{label}-after/results.json")
key = lambda s: hashlib.sha256(s.encode()).hexdigest()[:24]
rows, cat = [], collections.Counter()
for x, y in zip(b, a):
    for m in ("ast", "ascii", "yaml"):
        if x["results"][m]["stdout_sha256"] == y["results"][m]["stdout_sha256"]: continue
        k = key(x["source"])
        bb = open(f"{R}/runs/{label}-before/{k}.{m}", errors="replace").read(); aa = open(f"{R}/runs/{label}-after/{k}.{m}", errors="replace").read()
        order_only = bb != aa and sorted(bb.splitlines()) == sorted(aa.splitlines())
        svg_changed = x["results"]["svg"]["stdout_sha256"] != y["results"]["svg"]["stdout_sha256"]
        exit_change = (x["results"][m]["exit"], y["results"][m]["exit"])
        reason = ("order-only (HashMap iteration)" if order_only else
                  "mode exit changed with admission" if exit_change[0] != exit_change[1] else
                  "content change with SVG change" if svg_changed else "content change without SVG change")
        cat[(m, reason)] += 1
        rows.append(dict(source=x["source"], mode=m, reason=reason, exits=exit_change, svg_changed=svg_changed,
                         svg_outcome=[outcome(x), outcome(y)]))
for k, v in sorted(cat.items()): print(k, v)
json.dump(dict(categories={f"{k[0]}|{k[1]}": v for k, v in cat.items()}, rows=rows), open(f"{R}/{label}-model-changes.json", "w"), indent=1)
