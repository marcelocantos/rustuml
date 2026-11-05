"""Independent face-data check: read each Arial/Verdana face's hmtx advance (via cmap) for U+0020..U+007E with fontTools,
compare with the tip's const tables (parsed from text_render.rs) and with the maker's TSVs; also old f64 tables x 2048.
Usage: python3 -I hmtx_check.py <tip_text_render.rs> <merge_text_render.rs> <tsv_dir> <out.json>"""
import hashlib, json, re, sys, pathlib
from fontTools.ttLib import TTFont
tip, merge, tsvdir, outp = sys.argv[1:5]
src = open(tip).read(); msrc = open(merge).read()
def u16(name):
    m = re.search(rf"const {name}: \[u16; 95\] = \[(.*?)\];", src, re.S); return [int(x) for x in re.findall(r"\d+", m.group(1))]
def f64(name):
    m = re.search(rf"const {name}: \[f64; 95\] = \[(.*?)\];", msrc, re.S); return [float(x) for x in re.findall(r"[\d.]+", m.group(1))]
FILES = {("Arial", "plain"): "Arial.ttf", ("Arial", "bold"): "Arial Bold.ttf", ("Arial", "italic"): "Arial Italic.ttf", ("Arial", "bolditalic"): "Arial Bold Italic.ttf",
         ("Verdana", "plain"): "Verdana.ttf", ("Verdana", "bold"): "Verdana Bold.ttf", ("Verdana", "italic"): "Verdana Italic.ttf", ("Verdana", "bolditalic"): "Verdana Bold Italic.ttf"}
CONST = {"plain": "", "bold": "_BOLD", "italic": "_ITALIC", "bolditalic": "_BOLD_ITALIC"}
OLD = {"plain": "_WIDTH", "bold": "_BOLD_WIDTH"}
TSV = {("Arial", "plain"): "ArialMT.plain", ("Arial", "bold"): "Arial-BoldMT.bold", ("Arial", "italic"): "Arial-ItalicMT.italic", ("Arial", "bolditalic"): "Arial-BoldItalicMT.bolditalic",
       ("Verdana", "plain"): "Verdana.plain", ("Verdana", "bold"): "Verdana-Bold.bold", ("Verdana", "italic"): "Verdana-Italic.italic", ("Verdana", "bolditalic"): "Verdana-BoldItalic.bolditalic"}
out = {}
for (fam, st), fn in FILES.items():
    p = pathlib.Path("/System/Library/Fonts/Supplemental") / fn
    f = TTFont(str(p)); cmap = f.getBestCmap(); upm = f["head"].unitsPerEm
    adv = [f["hmtx"][cmap[c]][0] for c in range(0x20, 0x7F)]
    const = u16(fam.upper() + CONST[st] + "_ASCII")
    tsv = [float(l.split("\t")[3]) for l in open(pathlib.Path(tsvdir) / f"{TSV[(fam, st)]}.ascii.tsv") if l.startswith("A\t")]
    row = {"file": str(p), "file_sha256": hashlib.sha256(p.read_bytes()).hexdigest(), "units_per_em": upm,
           "psname": f["name"].getDebugName(6), "hmtx_eq_const": adv == const, "tsv_eq_hmtx": [int(x) for x in tsv] == adv,
           "const_minus_hmtx": {chr(0x20 + i): const[i] - adv[i] for i in range(95) if const[i] != adv[i]}}
    if st in OLD:
        old = f64(fam.upper() + OLD[st]); row["old_f64_x2048_eq_hmtx"] = [round(x * 2048, 6) for x in old] == [float(a) for a in adv]
        row["old_f64_exact_eighths"] = all(abs(x * 2048 - round(x * 2048)) < 1e-9 for x in old)
    out[f"{fam}/{st}"] = row
for fam in ["Arial", "Verdana"]:
    up = u16(fam.upper() + "_ASCII")
    for st in ["italic", "bolditalic"]:
        base = u16(fam.upper() + ("_BOLD" if st == "bolditalic" else "") + "_ASCII"); it = u16(fam.upper() + CONST[st] + "_ASCII")
        out[f"{fam}/{st}"]["differs_from_upright_slot"] = {chr(0x20 + i): [base[i], it[i]] for i in range(95) if base[i] != it[i]}
json.dump(out, open(outp, "w"), indent=1)
for k, v in out.items(): print(k, v["psname"], v["units_per_em"], "hmtx==const", v["hmtx_eq_const"], "tsv==hmtx", v["tsv_eq_hmtx"], v.get("old_f64_x2048_eq_hmtx", ""), v.get("differs_from_upright_slot", ""))
