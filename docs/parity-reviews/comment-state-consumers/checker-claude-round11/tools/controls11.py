"""Control-paired failures in C11A: case fails at tip while its same-consumer upright control MATCHes at tip.
Usage: python3 -I controls11.py <bx-eval.json> <batch-BX.json> <out.json>"""
import json, re, sys, collections
ev = json.load(open(sys.argv[1]))["per_case"]; rows = json.load(open(sys.argv[2]))
def control(n):
    p = n.split("_")
    if p[1] == "fc": return f"c11a_fc_{p[2]}_{p[3]}_p"
    if p[1].startswith("sz"): return f"c11a_{p[1]}_{p[2]}_vd_p"
    if p[1] == "cr": return f"c11a_cr_{p[2]}_ctl_{p[-1]}"
    if p[1] in ("st", "qu"): return f"c11a_{p[1]}_{p[2]}_{p[3]}_p"
    if p[1] == "of": return f"c11a_of_{p[2]}_{p[3]}_p"
    if p[1] == "sx": return f"c11a_sx_ctl_{p[-1]}"
    return None
out = collections.defaultdict(list); stat = collections.Counter()
for n, c in ev.items():
    k = control(n)
    if not k or k == n: continue
    ctl = ev[k]["tip"]["strict"] == "MATCH"; me = c["tip"]["strict"] == "MATCH"
    stat[(n.split("_")[1][:2], "ctl_match" if ctl else "ctl_fail", "match" if me else "fail")] += 1
    if ctl and not me:
        out[n.split("_")[1]].append({"name": n, "control": k, "merge": c["merge"]["strict"], "tip_ndiff": c["tip"]["ndiff"],
                                    "merge_ndiff": c["merge"]["ndiff"], "diff_head": rows[n].get("tip_diff_head", "")[:300]})
json.dump({"stat": {"/".join(k): v for k, v in sorted(stat.items())}, "controlled_failures": out}, open(sys.argv[3], "w"), indent=1)
for k, v in sorted(stat.items()): print(k, v)
for g, l in out.items():
    print("==", g, len(l))
    for x in l: print("  ", x["name"], "merge", x["merge"][:5], x["merge_ndiff"], "tip", x["tip_ndiff"], x["diff_head"][:160])
