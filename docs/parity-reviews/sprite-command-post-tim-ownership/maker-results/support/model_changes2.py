"""Explain every AST/ASCII/YAML stdout change between two capture runs (parameterised model_changes.py).

Usage: python3 -I model_changes2.py RUNS BEFORE_LABEL AFTER_LABEL OUT
Extra category: the only difference is the additive participant field `url_tooltip: None` (AST Debug)."""
import collections, hashlib, json, re, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from classify import outcome, load
runs, bl, al, out = sys.argv[1:5]
b, a = load(f"{runs}/{bl}/results.json"), load(f"{runs}/{al}/results.json")
key = lambda s: hashlib.sha256(s.encode()).hexdigest()[:24]
strip_tooltip = lambda text: re.sub(r"\n\s*url_tooltip: None,", "", text)
rows, cat = [], collections.Counter()
for x, y in zip(b, a):
    assert x["source"] == y["source"]
    for m in ("ast", "ascii", "yaml"):
        if x["results"][m]["stdout_sha256"] == y["results"][m]["stdout_sha256"]:
            continue
        k = key(x["source"])
        bb = open(f"{runs}/{bl}/{k}.{m}", errors="replace").read(); aa = open(f"{runs}/{al}/{k}.{m}", errors="replace").read()
        svg_changed = x["results"]["svg"]["stdout_sha256"] != y["results"]["svg"]["stdout_sha256"]
        exit_change = (x["results"][m]["exit"], y["results"][m]["exit"])
        if exit_change[0] != exit_change[1]:
            reason = "mode exit changed with admission"
        elif strip_tooltip(aa) == bb:
            reason = "additive url_tooltip: None field only"
        elif sorted(strip_tooltip(aa).splitlines()) == sorted(bb.splitlines()):
            reason = "order-only (HashMap iteration)"
        elif svg_changed:
            reason = "content change with SVG change"
        else:
            reason = "content change without SVG change"
        cat[(m, reason)] += 1
        rows.append(dict(source=x["source"], mode=m, reason=reason, exits=exit_change, svg_changed=svg_changed,
                         svg_outcome=[outcome(x), outcome(y)]))
for k, v in sorted(cat.items()):
    print(k, v)
Path(out).write_text(json.dumps(dict(categories={f"{k[0]}|{k[1]}": v for k, v in cat.items()}, rows=rows), indent=1))
