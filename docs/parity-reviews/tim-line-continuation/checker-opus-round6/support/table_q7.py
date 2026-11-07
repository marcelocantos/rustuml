"""Three-way table and worse lists for tip 7e13632cd from summary.json (T14.63 Welcome overlay + review-6 validity overlay).
Worse = lost_match, lost_rejection, lost_welcome_match, new_valid_error, new_panic, new_timeout.
Usage: python3 -I table_q7.py SUMMARY_JSON REVIEW6_WORSE_28af44ab3_JSON OUTDIR"""
import json, sys, collections
from pathlib import Path
s = json.loads(Path(sys.argv[1]).read_text()); r6 = json.loads(Path(sys.argv[2]).read_text()); out = Path(sys.argv[3]); out.mkdir(parents=True, exist_ok=True)
WORSE = ("lost_match", "lost_rejection", "lost_welcome_match", "new_valid_error", "new_panic", "new_timeout")
approved = {r["case"].split("/")[-1].removesuffix(".puml") for r in r6["rows"] if r["in_review5_approved"]}
inherited7 = {r["case"] for r in r6["rows"] if r["attribution"].startswith("inherited")}
named_inherited = {"r5b_num_unmarked_bs_end_only", "r5b_start_bs_files", "r5b_start_bs_creole", "r5b_nest_uml_content_bs_json"}
TIP = "7e13632cd"; BASES = ["c25ccbf3c", "9ed52f435", "9fc567332"]
short = lambda n: n.split("/")[-1].removesuffix(".puml")
table = []; worse = {b: [] for b in BASES}
for co, info in s.items():
    row = dict(cohort=co, n=info["n"], java_valid=info.get("java_valid"), tip_match=info["outcomes_t1463"][TIP].get("match", 0),
               welcome_match=info["outcomes_t1463"][TIP].get("welcome-match", 0))
    for b in BASES:
        d = info.get(f"t1463_vs_{b}", {})
        w = [(k, n) for k, v in d.items() if k in WORSE for n in v]
        row[f"worse_vs_{b}"] = len(w); row[f"other_vs_{b}"] = sum(len(v) for k, v in d.items() if k not in WORSE)
        for k, n in w: worse[b].append(dict(cohort=co, case=short(n), kind=k))
    table.append(row)
tot = {k: sum(r[k] or 0 for r in table) for k in table[0] if k != "cohort"}; tot["cohort"] = "TOTAL"; table.append(tot)
for b in BASES:
    for w in worse[b]:
        c = w["case"]
        w["in_review5_approved"] = c in approved; w["review6_inherited"] = c in inherited7; w["named_inherited"] = c in named_inherited
    (out / f"worse-vs-{b}.json").write_text(json.dumps(dict(total=len(worse[b]),
        outside_approved_named=[w for w in worse[b] if not (w["in_review5_approved"] or w["named_inherited"] or w["case"] == "r5b_start_bs_chen")],
        rows=worse[b]), indent=1))
(out / "three-way-table.json").write_text(json.dumps(table, indent=1))
print(f"{'cohort':10}{'n':>6}{'valid':>6}{'match':>6}{'welc':>5}" + "".join(f"{b:>16}" for b in BASES))
for r in table: print(f"{r['cohort']:10}{r['n']:>6}{str(r['java_valid'] or ''):>6}{r['tip_match']:>6}{r['welcome_match']:>5}" + "".join(f"{str(r[f'worse_vs_{b}'])+'/'+str(r[f'other_vs_{b}']):>16}" for b in BASES))
for b in BASES:
    o = [w for w in worse[b] if not (w["in_review5_approved"] or w["named_inherited"] or w["case"] == "r5b_start_bs_chen")]
    print("vs", b, "worse", len(worse[b]), "approved", sum(w["in_review5_approved"] for w in worse[b]), "named-inherited", sum(w["named_inherited"] for w in worse[b]),
          "chen", sum(w["case"] == "r5b_start_bs_chen" for w in worse[b]), "OUTSIDE", [(w["cohort"], w["case"], w["kind"]) for w in o])
print("approved not worse now:", sorted(approved - {w["case"] for w in worse["9fc567332"]}))
