"""Compare the TextLine run levels (text_line_levels, DEFAULT_LTR) with Java's run field on the checker's
DIRECTION_DEFAULT_LEFT_TO_RIGHT inputs. Java's TextLine uses the runs only when requiresBidi && !isLeftToRight.
Usage: python3 -I textline_compare.py <bidi-input.txt> <rust dump> <bidi-java-levels.txt> <out.json>"""
import json, sys
inp = open(sys.argv[1]).read().splitlines(); rs = open(sys.argv[2]).read().splitlines(); jv = open(sys.argv[3]).read().splitlines()[1:]
assert len(inp) == len(rs) == len(jv) == 242508
res = {"default_ltr_inputs": 0, "textline_uses_runs": 0, "run_level_differences": 0, "other_field_differences": 0, "examples": []}
for i, line in enumerate(inp):
    if not line.startswith("D;"):
        continue
    res["default_ltr_inputs"] += 1
    rb, ltr, pl, lv, tl = rs[i].split(";"); jrb, jltr, jpl, jlv, jrun = jv[i].split(";")
    if (rb, ltr, pl, lv) != (jrb, jltr, jpl, jlv):
        res["other_field_differences"] += 1
    expected = jrun if (jrb == "1" and jltr == "0") else "-"
    if expected != "-":
        res["textline_uses_runs"] += 1
    if tl != expected:
        res["run_level_differences"] += 1
        if len(res["examples"]) < 20:
            res["examples"].append({"line": i, "input": line, "rust": tl, "java_runs": jrun})
json.dump(res, open(sys.argv[4], "w"), indent=1)
print({k: v for k, v in res.items() if k != "examples"})
