import collections, json, sys
sys.path.insert(0, sys.argv[1] + "/support")
from classify import outcome, load
R = sys.argv[1]
m = {r["name"]: r for r in json.load(open(R + "/fresh-manifest-classified.json"))}
b = {r["name"]: r for r in load(R + "/runs/fresh-before/results.json")}
a = {r["name"]: r for r in load(R + "/runs/fresh-after/results.json")}
tab = collections.defaultdict(collections.Counter); detail = collections.defaultdict(list)
for n, r in m.items():
    ob, oa = outcome(b[n]), outcome(a[n]); mech = r["mechanism"] or "supplement"
    if r["name"].startswith("c75s_"): mech = "supplement:" + mech
    tab[mech][(r["observed_class"], ob, oa)] += 1
    if ob != oa: detail[mech].append((n, ob, oa))
out = {}
for mech, c in sorted(tab.items()):
    print("##", mech)
    for k, v in sorted(c.items()): print("   ", k, v)
    out[mech] = {f"{k[0]}|{k[1]}->{k[2]}": v for k, v in c.items()}
json.dump(dict(tables=out, transitions={k: v for k, v in detail.items()}), open(R + "/fresh-tables.json", "w"), indent=1)
