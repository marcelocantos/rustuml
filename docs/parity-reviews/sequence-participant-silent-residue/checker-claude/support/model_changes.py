"""Categorise AST/YAML/ASCII changes between before and after replays by line-multiset difference.
AST is Rust Debug text, YAML is serde_yml text. A change is 'order-only' when the line multisets are equal;
'url_tooltip-additive' when the only added lines are `url_tooltip: None,`; otherwise the distinct added/removed
line shapes are recorded (digits collapsed) so every category is inspectable."""
import collections, hashlib, json, re, sys
from pathlib import Path
lab, bdir, adir = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
B = json.loads((bdir / "results.json").read_text())["rows"]; A = json.loads((adir / "results.json").read_text())["rows"]
key = lambda s: hashlib.sha256(s.encode()).hexdigest()[:24]
cats = collections.Counter(); ex = collections.defaultdict(list)
def shape(l): return re.sub(r"\d+", "N", l.strip())[:120]
for b, a in zip(B, A):
    k = key(b["source"]); svg_same = b["results"]["svg"]["stdout_sha256"] == a["results"]["svg"]["stdout_sha256"]
    for m in ("ast", "yaml", "ascii"):
        if b["results"][m]["stdout_sha256"] == a["results"][m]["stdout_sha256"]: continue
        if b["results"][m]["exit"] != a["results"][m]["exit"]: c = f"exit {b['results'][m]['exit']}->{a['results'][m]['exit']}"
        else:
            lb = collections.Counter((bdir / f"{k}.{m}").read_text().splitlines()); la = collections.Counter((adir / f"{k}.{m}").read_text().splitlines())
            add = la - lb; rem = lb - la
            if not add and not rem: c = "order-only"
            else:
                sa = sorted({shape(l) for l in add.elements()}); sr = sorted({shape(l) for l in rem.elements()})
                if sa == ["url_tooltip: None,"] and not sr: c = "url_tooltip-additive"
                else: c = "add[" + " | ".join(sa)[:240] + "] rem[" + " | ".join(sr)[:240] + "]"
        c = f"{m}: {c} (svg {'same' if svg_same else 'changed'})"
        cats[c] += 1; ex[c].append(b["source"])
out = dict(label=lab, categories=dict(cats.most_common()), examples={c: v[:5] for c, v in ex.items()})
print(json.dumps(out, indent=1)[:9000])
if len(sys.argv) > 4: Path(sys.argv[4]).write_text(json.dumps(dict(out, all=dict(ex)), indent=1))
