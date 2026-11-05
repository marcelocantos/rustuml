"""Per pair: sum of textLength in mark case minus control, Java vs base/merge/tip. Usage: deltas9.py <work> <label> <out.json>"""
import json, re, sys, pathlib
W = pathlib.Path(sys.argv[1]); L = sys.argv[2]; d = json.load(open(W / f"batch-{L}.json")); O = W / f"out-{L}"
tl = lambda n, t: sum(float(x) for x in re.findall(r'textLength="([^"]*)"', (O / f"{n}.{t}.svg").read_text(errors="replace")))
res = {}
for n in d:
    if n + "_ctrl" not in d: continue
    row = {"strict": {t: [d[n][t + "_strict"], d[n + "_ctrl"][t + "_strict"]] for t in ["base", "merge", "tip"]}}
    for t in ["java", "base", "merge", "tip"]: row[f"delta_{t}"] = round(tl(n, t) - tl(n + "_ctrl", t), 4)
    for t in ["base", "merge", "tip"]: row[f"{t}_delta_eq_java"] = abs(row[f"delta_{t}"] - row["delta_java"]) < 0.002
    res[n] = row
json.dump(res, open(sys.argv[3], "w"), indent=1)
for n, r in res.items():
    print(f"{n:34s} J{r['delta_java']:9.4f} b{r['delta_base']:9.4f} m{r['delta_merge']:9.4f} t{r['delta_tip']:9.4f} {'=' if r['tip_delta_eq_java'] else 'X'} strict(m/c) " + " ".join(f"{t[0]}:{'M' if a=='MATCH' else '-'}{'M' if b=='MATCH' else '-'}" for t, (a, b) in r['strict'].items()))
