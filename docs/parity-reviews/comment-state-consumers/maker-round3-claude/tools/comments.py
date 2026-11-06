"""Exact comment-list oracle: the ordered <!--...--> list of each output versus the Java golden.

Usage: python3 -I comments.py <repo> <out-before> <out-after> <out.json>
"""
import json, pathlib, re, sys

C = re.compile(r"<!--(.*?)-->", re.S)
repo, ob, oa, out = sys.argv[1:]
repo = pathlib.Path(repo)
st = json.load(open(pathlib.Path(oa) / "status.json"))
before_st = json.load(open(pathlib.Path(ob) / "status.json"))

def comments(path):
    try:
        return C.findall(path.read_text(errors="replace"))
    except FileNotFoundError:
        return None

res = {}
for rel in st:
    java = comments((repo / rel).with_suffix(".svg"))
    b = comments((pathlib.Path(ob) / rel).with_suffix(".svg")) if rel in before_st else None
    a = comments((pathlib.Path(oa) / rel).with_suffix(".svg"))
    res[rel] = (b == java, a == java)
n = len(res)
before = sum(x for x, _ in res.values())
after = sum(y for _, y in res.values())
lost = sorted(k for k, (x, y) in res.items() if x and not y)
gained = sorted(k for k, (x, y) in res.items() if y and not x)
json.dump({"pairs": n, "exact_before": before, "exact_after": after, "gained": gained, "lost": lost},
          open(out, "w"), indent=1)
print(f"{n} pairs; exact before {before}, after {after}; gained {len(gained)}, lost {len(lost)}")
for k in lost:
    print("  LOST", k)
