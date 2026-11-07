"""Compact three-way table from summary.json (T14.63 overlay). Worse = lost match, lost rejection, lost Welcome match,
new valid-input error, new panic, new timeout. Usage: python3 -I table_r5c.py SUMMARY OUT_JSON"""
import json, sys
s = json.load(open(sys.argv[1])); AFTER = "c4b5a8436"; BEF = ["9dc1af5f9", "9ed52f435", "32684f81e"]
WORSE = {"lost_match", "lost_rejection", "lost_welcome_match", "new_valid_error", "new_panic", "new_timeout"}
out = {}
for co, i in s.items():
    o = i["outcomes_t1463"][AFTER]; row = dict(n=i["n"], tip_match=o.get("match", 0), tip_welcome_match=o.get("welcome-match", 0), java_welcome=len(i["java_welcome"]))
    for b in BEF:
        d = i.get(f"t1463_vs_{b}")
        if d is None: continue
        worse = sorted(x for k, v in d.items() if k in WORSE for x in v); better = sorted(x for k, v in d.items() if k not in WORSE for x in v)
        other = {k: len(v) for k, v in d.items() if k not in WORSE}
        row[b] = dict(worse=len(worse), better_or_other=len(better), worse_cases=worse, worse_by_kind={k: v for k, v in d.items() if k in WORSE}, other_kinds=other,
                      svg_byte_changes=i[f"vs_{b}"]["svg_byte_changes"], model_changes=i[f"vs_{b}"]["model_changes"])
    row["panics_tip"] = i["panics"][AFTER]; row["timeouts"] = {b: i["timeouts"][b] for b in i["timeouts"]}
    out[co] = row
json.dump(out, open(sys.argv[2], "w"), indent=1)
print(f"{'cohort':9}{'n':>6}{'match':>7}{'wm':>4}{'jw':>4} | " + " | ".join(f"vs {b} w/g svg" for b in BEF))
for co, r in out.items():
    print(f"{co:9}{r['n']:>6}{r['tip_match']:>7}{r['tip_welcome_match']:>4}{r['java_welcome']:>4} | " +
          " | ".join(f"{r[b]['worse']}/{r[b]['better_or_other']} {r[b]['svg_byte_changes']}" if b in r else "-" for b in BEF), " panics", r["panics_tip"], "to", {k: v for k, v in r["timeouts"].items() if v})
