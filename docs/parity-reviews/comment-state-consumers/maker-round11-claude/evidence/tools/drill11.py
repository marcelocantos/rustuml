"""Mutation drill of the round-11 gates on the working tree (each mutant applied, rustuml-render lib tests run,
file restored with git checkout). Usage: python3 -I drill11.py <repo> <target_dir> <out.json>"""
import json, pathlib, re, subprocess, sys
R = pathlib.Path(sys.argv[1]); T = sys.argv[2]; OUT = sys.argv[3]
TR = "crates/rustuml-render/src/text_render.rs"
M = {
 "D0_unmodified": None,
 "D1_verdana_router_drops_italic": (TR, "            italic,\n            ChainFamily::Verdana,\n", "            false,\n            ChainFamily::Verdana,\n"),
 "D2_slot_italic_reads_plain": (TR, "            (false, true) => self.italic,", "            (false, true) => self.plain,"),
 "D3_slot_bold_italic_reads_bold": (TR, "            (true, true) => self.bold_italic,", "            (true, true) => self.bold,"),
 "D4_verdana_italic_table_is_regular": (TR, "    italic: &VERDANA_ITALIC_ASCII,", "    italic: &VERDANA_ASCII,"),
 "D5_verdana_bold_italic_table_is_bold": (TR, "    bold_italic: &VERDANA_BOLD_ITALIC_ASCII,", "    bold_italic: &VERDANA_BOLD_ASCII,"),
 "D6_arial_router_drops_italic": (TR, "            italic,\n            ChainFamily::Arial,\n", "            false,\n            ChainFamily::Arial,\n"),
 "D7_one_verdana_italic_unit_off": (TR, "const VERDANA_ITALIC_ASCII: [u16; 95] = [\n    720,", "const VERDANA_ITALIC_ASCII: [u16; 95] = [\n    721,"),
 "D8_extraction_file_edited": ("tools/font-chain-reference/ascii-advances-round11/Verdana-Italic.italic.ascii.tsv", "A\t41\t36\t1398.0", "A\t41\t36\t1400.0"),
}
res = {}
for name, mut in M.items():
    if mut:
        f, old, new = mut; p = R / f; s = p.read_text()
        assert s.count(old) == 1, (name, s.count(old)); p.write_text(s.replace(old, new))
    try:
        r = subprocess.run(["cargo", "test", "-q", "-p", "rustuml-render", "--lib"], cwd=R, capture_output=True, text=True,
                           env={**__import__("os").environ, "CARGO_TARGET_DIR": T})
    finally:
        if mut: subprocess.run(["git", "checkout", "--", mut[0]], cwd=R, check=True)
    failed = sorted(set(re.findall(r"^    (\S+)$", r.stdout, re.M)))
    m = re.search(r"test result: \w+\. (\d+) passed; (\d+) failed", r.stdout)
    res[name] = {"mutant": mut and {"file": mut[0], "old": mut[1], "new": mut[2]}, "exit": r.returncode,
                 "counts": m.groups() if m else None, "failed": failed, "caught": bool(failed) if mut else None,
                 "build_error": "error[" in r.stderr}
    print(name, r.returncode, res[name]["counts"], failed)
json.dump(res, open(OUT, "w"), indent=1)
