"""For Java-valid mismatches, compare the multiset of paint attributes (fill and stroke colours) between Java and Rust SVG.

Usage: python3 -I fill_audit.py RESULTS_JSON REFDIR OUT NAME...  (NAME list or @mechanism)"""
import collections, hashlib, json, re, sys
from pathlib import Path
results, refdir, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
rows = json.loads(results.read_text())["rows"]
names = set(sys.argv[4:])
paint = re.compile(r'(?:fill|stroke)="(#[0-9A-Fa-f]{3,8}|[a-zA-Z]+)"|stroke:(#[0-9A-Fa-f]{3,8})')
key = lambda item: hashlib.sha256(item.encode()).hexdigest()[:24]
report = []
for row in rows:
    if row["name"] not in names:
        continue
    java = (refdir / row["golden"]).read_text(errors="replace")
    rust = (results.parent / f"{key(row['source'])}.svg").read_text(errors="replace")
    pj = collections.Counter(a or b for a, b in paint.findall(java))
    pr = collections.Counter(a or b for a, b in paint.findall(rust))
    report.append(dict(name=row["name"], paint_equal=pj == pr,
                       java_only=dict(pj - pr), rust_only=dict(pr - pj)))
out.write_text(json.dumps(report, indent=1))
print(sum(r["paint_equal"] for r in report), "of", len(report), "paint-equal")
for r in report:
    if not r["paint_equal"]:
        print(" ", r["name"], r["java_only"], r["rust_only"])
