"""Exact viewBox census over the sweep (merge vs tip) for every source, plus the list of outputs changed merge->tip
with their strict status. Usage: python3 -I sweep_vb.py <work> <repo> <out.json>"""
import json, re, sys, pathlib
W, R = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]); VB = re.compile(rb'viewBox="([^"]*)"')
st = {s: json.load(open(W / f"sweep-{s}/status.json")) for s in ("merge", "tip")}
cmp = {}
for line in open(W / "cmp.tsv"):
    a, r, *_ = line.rstrip("\n").split("\t")
    for s in ("merge", "tip"):
        if f"/sweep-{s}/" in a: cmp[(s, a.split(f"/sweep-{s}/", 1)[1][:-4] + ".puml")] = r
out = {"viewbox_exact": {}, "viewbox_lost": [], "viewbox_gained": [], "changed": {}}
ex = {"merge": set(), "tip": set()}
for k in st["tip"]:
    j = VB.findall((R / k).with_suffix(".svg").read_bytes())
    for s in ex:
        if VB.findall((W / f"sweep-{s}" / k).with_suffix(".svg").read_bytes()) == j: ex[s].add(k)
    if st["merge"][k]["stdout_sha256"] != st["tip"][k]["stdout_sha256"]:
        out["changed"][k] = {"merge": cmp[("merge", k)], "tip": cmp[("tip", k)]}
for coh in ("test-diagrams/golden/", "test-diagrams/perturbations/"):
    out["viewbox_exact"][coh] = {s: sum(1 for k in v if k.startswith(coh)) for s, v in ex.items()}
out["viewbox_lost"] = sorted(ex["merge"] - ex["tip"]); out["viewbox_gained"] = sorted(ex["tip"] - ex["merge"])
json.dump(out, open(sys.argv[3], "w"), indent=1)
print(out["viewbox_exact"], "lost", out["viewbox_lost"], "gained", len(out["viewbox_gained"]))
import collections; print(collections.Counter(f"{v['merge']}->{v['tip']}" for v in out["changed"].values()))
for k, v in out["changed"].items():
    if v["tip"] != "MATCH": print(" ", k.split("perturbations/")[1], v)
