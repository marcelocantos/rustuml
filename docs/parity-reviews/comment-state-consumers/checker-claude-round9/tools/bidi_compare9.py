"""Checker r9: compare tip java_bidi dump with Java's differential. Usage: bidi_compare9.py <input> <rust-dump> <java> <out.json>"""
import json, sys
inp = open(sys.argv[1]).read().splitlines(); rs = open(sys.argv[2]).read().splitlines(); jv = open(sys.argv[3]).read().splitlines()
hdr, jv = jv[0], jv[1:]; assert len(inp) == len(rs) == len(jv)
parts = {"A_conformance": range(0, 91707), "B_random": range(91707, 241707), "C_stack": range(241707, len(inp))}
res = {"java_header": hdr, "parts": {}}
for name, rng in parts.items():
    c = {k: 0 for k in ["lines", "java_exc", "requiresBidi", "isLTR", "paraLevel", "levelAt", "run_levels_all_dirs", "D_lines", "D_textline_used", "D_textline_diff"]}
    ex = []
    for i in rng:
        c["lines"] += 1; j = jv[i]
        if j.startswith("EXC"): c["java_exc"] += 1; continue
        rb, ltr, pl, lv, run, tl = rs[i].split(";"); jrb, jltr, jpl, jlv, jrun = j.split(";")
        d = []
        if rb != jrb: d.append("requiresBidi")
        if ltr != jltr: d.append("isLTR")
        if pl != jpl: d.append("paraLevel")
        if lv != jlv: d.append("levelAt")
        if run != jrun: d.append("run_levels_all_dirs")
        if inp[i].startswith("D;"):
            c["D_lines"] += 1
            jt = jrun if (jrb == "1" and jltr == "0") else "-"
            if jt != "-": c["D_textline_used"] += 1
            if tl != jt: d.append("D_textline_diff")
        for k in d: c[k] += 1
        if d and len(ex) < 20: ex.append([i, inp[i], rs[i], j, d])
    c["examples"] = ex; res["parts"][name] = c
json.dump(res, open(sys.argv[4], "w"), indent=1)
for n, c in res["parts"].items(): print(n, {k: v for k, v in c.items() if k != "examples"})
