import json, re, pathlib, sys, collections
W = pathlib.Path(sys.argv[1]); mode = sys.argv[2]  # out or outroot
C = re.compile(r"<!--(.*?)-->", re.S)
st = json.load(open(W/f"{mode}-after/status.json"))
res = {}
for k in st:
    j = C.findall((W/"src-after/test-diagrams"/k).with_suffix(".svg").read_text(errors="replace"))
    r = {s: C.findall((W/f"{mode}-{s}"/k).with_suffix(".svg").read_text(errors="replace")) for s in ("before","after")}
    res[k] = (r["before"] == j, r["after"] == j)
n = len(res); b = sum(x for x, _ in res.values()); a = sum(y for _, y in res.values())
lost = sorted(k for k, (x, y) in res.items() if x and not y); gained = sorted(k for k, (x, y) in res.items() if y and not x)
print(f"{mode}: {n} pairs; exact before {b}, after {a}; gained {len(gained)}, lost {len(lost)}")
for k in lost: print("  LOST", k)
json.dump({"pairs": n, "exact_before": b, "exact_after": a, "gained": gained, "lost": lost}, open(W/f"comments_{mode}.json", "w"), indent=1)
