"""Oracle for the font-chain model: compare tools/FontChainExtract.swift output for Times New Roman with the
Java AWT table crates/rustuml-render/src/times_new_roman_unicode.rs (Font.getStringBounds of every scalar
Java defines, alone; MISSING_FIXED_UNITS marks the missing glyph).
Usage: python3 -I font_chain_validate.py <TimesNewRomanPSMT.tsv> <times_new_roman_unicode.rs> <out.json>"""
import collections, json, os, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import font_chain_model as m
hdr, runs = m.load(sys.argv[1])
src = open(sys.argv[2]).read()
MISSING = int(re.search(r"MISSING_FIXED_UNITS: \[u32; 4\] = \[(\d+)", src).group(1))
rs = [(int(a, 16), int(b, 16), int(c.split(",")[0])) for a, b, c in
      re.findall(r"\(\s*0x([0-9A-F]+),\s*0x([0-9A-F]+),\s*\[([\d, ]+)\]", src)]
stats = collections.Counter(); ex = collections.defaultdict(list)
for a, b, v in rs:
    for cp in range(a, b + 1):
        java = "missing" if v == MISSING else ("zero" if v == 0 else "advance")
        p = m.predict(runs, cp)
        ok = (java == "missing") == (p == "missing") and (p != "ignorable" or java == "zero")
        key = f"java={java} model={p} {'agree' if ok else 'DISAGREE'} {'bmp' if cp < 0x10000 else 'supplementary'}"
        stats[key] += 1
        if not ok and len(ex[key]) < 60: ex[key].append(f"U+{cp:04X}")
tot = sum(stats.values()); agree = sum(v for k, v in stats.items() if " agree " in k)
out = {"oracle": "times_new_roman_unicode.rs (Java AWT extraction, PhysicalFontMetricsExtract --rust-unicode-ranges)",
       "extract_header": hdr, "java_defined_scalars": tot, "agree": agree, "disagree": tot - agree,
       "classes": dict(sorted(stats.items(), key=lambda kv: -kv[1])), "disagreement_examples": ex}
json.dump(out, open(sys.argv[3], "w"), indent=1)
print("scalars", tot, "agree", agree, "disagree", tot - agree)
