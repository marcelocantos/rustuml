"""Checker gate drill: the maker's 8 mutations (drill11.json) plus checker mutations, each on a pristine private copy of the
tip tree; `cargo test -p rustuml-render --lib` per mutant. Usage: python3 -I drill11c.py <pristine_tree> <work_tree> <maker drill11.json> <out.json>"""
import hashlib, json, re, shutil, subprocess, sys, pathlib
P, T, MK, OUT = map(pathlib.Path, sys.argv[1:5])
TR = "crates/rustuml-render/src/text_render.rs"; TSV = "tools/font-chain-reference/ascii-advances-round11"
muts = {k: [v["mutant"]] if v["mutant"] else [] for k, v in json.load(open(MK)).items()}
src = (P / TR).read_text()
def const_body(name): return re.search(rf"const {name}: \[u16; 95\] = \[(.*?)\];", src, re.S).group(1)
vi, vbi = const_body("VERDANA_ITALIC_ASCII"), const_body("VERDANA_BOLD_ITALIC_ASCII")
ai, abi = const_body("ARIAL_ITALIC_ASCII"), const_body("ARIAL_BOLD_ITALIC_ASCII")
def drop_glyph(body, idx):  # remove the idx-th entry and append a copy of the last one (array length stays 95)
    nums = re.findall(r"\d+", body); nums = nums[:idx] + nums[idx + 1:] + [nums[-1]]
    return "\n    " + ", ".join(nums) + ",\n"
C = {
 "K1_swap_verdana_italic_and_bold_italic_slots": [{"file": TR, "old": "    italic: &VERDANA_ITALIC_ASCII,\n    bold_italic: &VERDANA_BOLD_ITALIC_ASCII,",
                                                   "new": "    italic: &VERDANA_BOLD_ITALIC_ASCII,\n    bold_italic: &VERDANA_ITALIC_ASCII,"}],
 "K2_swap_verdana_italic_and_bold_italic_rows": [{"file": TR, "old": f"const VERDANA_ITALIC_ASCII: [u16; 95] = [{vi}];", "new": f"const VERDANA_ITALIC_ASCII: [u16; 95] = [{vbi}];"},
                                                 {"file": TR, "old": f"const VERDANA_BOLD_ITALIC_ASCII: [u16; 95] = [{vbi}];", "new": f"const VERDANA_BOLD_ITALIC_ASCII: [u16; 95] = [{vi}];"}],
 "K3_swap_arial_italic_and_bold_italic_rows": [{"file": TR, "old": f"const ARIAL_ITALIC_ASCII: [u16; 95] = [{ai}];", "new": f"const ARIAL_ITALIC_ASCII: [u16; 95] = [{abi}];"},
                                               {"file": TR, "old": f"const ARIAL_BOLD_ITALIC_ASCII: [u16; 95] = [{abi}];", "new": f"const ARIAL_BOLD_ITALIC_ASCII: [u16; 95] = [{ai}];"}],
 "K4_drop_glyph_D_from_verdana_italic_const": [{"file": TR, "old": f"const VERDANA_ITALIC_ASCII: [u16; 95] = [{vi}];", "new": f"const VERDANA_ITALIC_ASCII: [u16; 95] = [{drop_glyph(vi, 0x44 - 32)}];"}],
 "K5_drop_row_D_from_verdana_italic_tsv": [{"file": f"{TSV}/Verdana-Italic.italic.ascii.tsv", "drop_line_prefix": "A\t44\t"}],
 "K6_drop_row_D_tsv_with_sums_and_pin_rehashed": [{"file": f"{TSV}/Verdana-Italic.italic.ascii.tsv", "drop_line_prefix": "A\t44\t"}, {"rehash": True}],
 "K7_router_slot_arguments_swapped": [{"file": TR, "old": "    let table = faces.slot(bold, italic);", "new": "    let table = faces.slot(italic, bold);"}],
 "K8_chain_face_drops_italic_only": [{"file": TR, "old": "        ChainFace::for_style(family, bold, italic),\n        |c| {", "new": "        ChainFace::for_style(family, bold, false),\n        |c| {"}],
 "K9_consistent_forgery_const_tsv_sums_pin": [{"file": TR, "old": "const VERDANA_ITALIC_ASCII: [u16; 95] = [\n    720,", "new": "const VERDANA_ITALIC_ASCII: [u16; 95] = [\n    721,"},
                                              {"file": f"{TSV}/Verdana-Italic.italic.ascii.tsv", "old": "A\t20\t3\t720.0", "new": "A\t20\t3\t721.0"}, {"rehash": True}],
}
muts.update(C); res = {}
for name, ms in muts.items():
    if T.exists(): shutil.rmtree(T)
    shutil.copytree(P, T, symlinks=True)
    applied = True
    for m in ms:
        if m.get("rehash"):
            d = T / TSV; lines = []
            for line in (d / "SHA256SUMS").read_text().splitlines():
                h, n = line.split("  ", 1); lines.append(f"{hashlib.sha256((d / n).read_bytes()).hexdigest()}  {n}")
            (d / "SHA256SUMS").write_text("\n".join(lines) + "\n")
            new = hashlib.sha256((d / "SHA256SUMS").read_bytes()).hexdigest(); f = T / TR; s = f.read_text()
            s2 = re.sub(r'(const SHA256SUMS_SHA256: &str =\s*")[0-9a-f]{64}(")', rf"\g<1>{new}\g<2>", s); applied &= s2 != s; f.write_text(s2); continue
        f = T / m["file"]; s = f.read_text()
        if "drop_line_prefix" in m:
            s2 = "".join(l for l in s.splitlines(True) if not l.startswith(m["drop_line_prefix"]))
        else:
            s2 = s.replace(m["old"], m["new"], 1)
        applied &= s2 != s; f.write_text(s2)
    p = subprocess.run(["cargo", "test", "-j", "6", "-p", "rustuml-render", "--lib"], cwd=T, capture_output=True, text=True,
                       env={**__import__("os").environ, "CARGO_TARGET_DIR": str(T.parent / "tgt-drill")})
    failed = re.findall(r"^    (\S+)$", p.stdout.split("failures:")[-1], re.M) if "failures:" in p.stdout else []
    counts = re.findall(r"(\d+) passed; (\d+) failed", p.stdout)
    res[name] = {"mutations": ms, "applied": applied, "exit": p.returncode, "counts": counts[-1] if counts else None,
                 "failed": sorted(set(failed)), "build_error": "error[" in p.stderr, "caught": (p.returncode != 0) if ms else None}
    print(name, "applied" if applied else "NOT-APPLIED", p.returncode, res[name]["counts"], res[name]["failed"], "BUILD-ERROR" if res[name]["build_error"] else "")
json.dump(res, open(OUT, "w"), indent=1)
