"""Compare java bidi-java-levels.txt with rust-tip-levels.txt. Usage: python3 -I compare.py <input> <rust> <java> <out.json>"""
import json, sys
inp = open(sys.argv[1]).read().splitlines(); rs = open(sys.argv[2]).read().splitlines(); jv = open(sys.argv[3]).read().splitlines()
hdr = jv[0]; jv = jv[1:]; assert len(inp) == len(rs) == len(jv), (len(inp), len(rs), len(jv))
parts = {"A_conformance": range(0, 91707), "B_random": range(91707, 241707), "C_stack": range(241707, len(inp))}
res = {"java_header": hdr, "parts": {}}
for name, rng in parts.items():
    c = {"lines": len(rng), "java_exceptions": 0, "requiresBidi_diff": 0, "isLTR_diff": 0, "paraLevel_diff": 0,
         "levelAt_diff": 0, "textline_levels_diff": 0, "examples": []}
    for i in rng:
        j = jv[i]
        if j.startswith("EXC"): c["java_exceptions"] += 1; c["examples"].append((i, inp[i], rs[i], j)); continue
        rb, ltr, pl, lv, tl = rs[i].split(";"); jrb, jltr, jpl, jlv, jrun = j.split(";")
        d = False
        if rb != jrb: c["requiresBidi_diff"] += 1; d = True
        if ltr != jltr: c["isLTR_diff"] += 1; d = True
        if pl != jpl: c["paraLevel_diff"] += 1; d = True
        if lv != jlv: c["levelAt_diff"] += 1; d = True
        # TextLine: levels used only when requiresBidi && !isLeftToRight (Java: run levels)
        jt = jrun if (jrb == "1" and jltr == "0") else "-"
        if tl != jt: c["textline_levels_diff"] += 1; d = True
        if d and len(c["examples"]) < 40: c["examples"].append((i, inp[i], rs[i], j))
    res["parts"][name] = c
json.dump(res, open(sys.argv[4], "w"), indent=1)
for n, c in res["parts"].items(): print(n, {k: v for k, v in c.items() if k != "examples"})
