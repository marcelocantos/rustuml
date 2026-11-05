"""Checker round-8 mutation drill of the drift gate and the bidi/blocklist unit tests.
Usage: python3 -I drift_drill8.py <git-tracked tip copy> <logdir>
Each mutation is applied to the copy, `cargo test --release -p rustuml-render --lib -- font_chain java_bidi`
runs, the exit status and failing tests are recorded, and the copy is restored (git checkout/clean of that private copy)."""
import gzip, hashlib, json, pathlib, re, subprocess, sys
T = pathlib.Path(sys.argv[1]); LOG = pathlib.Path(sys.argv[2]); LOG.mkdir(exist_ok=True)
R = T / "crates/rustuml-render/src"; S = T / "tools/font-chain-reference/shaping-sources"
P = T / "tools/font-chain-reference/colossus-primary-faces"
sha = lambda b: hashlib.sha256(b).hexdigest()
def sub(path, old, new, count=1):
    t = path.read_text(); assert old in t, (path, old); path.write_text(t.replace(old, new, count))
def resum(rel):
    """Rewrite the SHA256SUMS line of a pinned file and the pinned SUMS constant (a consistent forgery)."""
    sums = (S / "SHA256SUMS").read_text().splitlines()
    sums = [f"{sha((S / rel).read_bytes())}  {rel}" if l.endswith("  " + rel) else l for l in sums]
    (S / "SHA256SUMS").write_text("\n".join(sums) + "\n")
    fc = R / "font_chain.rs"; t = fc.read_text()
    t = re.sub(r'(SHAPING_SOURCES_SHA256SUMS_SHA256: &str =\s*")[0-9a-f]{64}', lambda m: m.group(1) + sha((S / "SHA256SUMS").read_bytes()), t)
    fc.write_text(t)
def m09():
    p = S / "unicode-18.0.0/UnicodeData.txt.gz"; text = gzip.decompress(p.read_bytes()).decode()
    text = "\n".join(l for l in text.split("\n") if not l.startswith("1D167;")); p.write_bytes(gzip.compress(text.encode(), 9, mtime=0)); resum("unicode-18.0.0/UnicodeData.txt.gz")
def m10():
    p = S / "jdk-21.0.5-ga/ubidi.icu"; b = bytearray(p.read_bytes()); b[-40] ^= 0x01; p.write_bytes(bytes(b)); resum("jdk-21.0.5-ga/ubidi.icu")
def m08():
    sub(S / "harfbuzz-15.0.0/hb-ot-layout.cc", "    case HB_CODEPOINT_ENCODE3 (832, 7324, 47162):\n", ""); resum("harfbuzz-15.0.0/hb-ot-layout.cc")
M = {
 "M01_bidi_class_nsm_run_to_on": lambda: sub(R / "java_bidi_data.rs", "(0x591, 0x5BD, 17)", "(0x591, 0x5BD, 10)"),
 "M02_bracket_pair_changed": lambda: sub(R / "java_bidi_data.rs", "(0x28, 0x29, true)", "(0x28, 0x5D, true)"),
 "M03_mongolian_use_to_arabic": lambda: sub(R / "font_chain_unicode.rs", "(27, ShaperGroup::Use)", "(27, ShaperGroup::Arabic)"),
 "M04_tahoma_blocklist_triple_changed": lambda: sub(R / "font_chain_unicode.rs", "(832, 7324, 47162)", "(832, 7324, 47163)"),
 "M05_generated_courier_gsub_arab_dropped": lambda: sub(R / "font_chain_coverage.rs", 'gsub_scripts: &[*b"arab", *b"hebr", *b"latn"],\n        gdef_length: 588', 'gsub_scripts: &[*b"hebr", *b"latn"],\n        gdef_length: 588'),
 "M06_extraction_courier_gdef_length_sums_untouched": lambda: sub(P / "CourierNewPSMT.own.tsv", "gdef_length=588", "gdef_length=589"),
 "M07_pinned_hb_layout_appended_sums_untouched": lambda: (S / "harfbuzz-15.0.0/hb-ot-layout.cc").write_text((S / "harfbuzz-15.0.0/hb-ot-layout.cc").read_text() + "// x\n"),
 "M08_pinned_hb_layout_tahoma_case_removed_sums_and_constant_forged": m08,
 "M09_ucd18_u1d167_removed_sums_and_constant_forged": m09,
 "M10_ubidi_icu_byte_flipped_sums_and_constant_forged": m10,
 "M11_monkeyman_record_courier_hash_changed": lambda: sub(P / "monkeyman-round7-font-hash.txt", "c886ca3172d8ce9189976992d145ec090500a4ff516ecb9d4df56b1a78336247", "c886ca3172d8ce9189976992d145ec090500a4ff516ecb9d4df56b1a78336248"),
 "M12_code_requires_bidi_without_AN": lambda: sub(R / "java_bidi.rs", "flag(RLO) | flag(AN);", "flag(RLO);"),
 "M13_code_no_bracket_pairing": lambda: sub(R / "java_bidi.rs", "        let dir_prop = self.dir_props[position];\n        let mut new_prop;", "        let dir_prop = self.dir_props[position];\n        if dir_prop != 255 { return; }\n        let mut new_prop;"),
 "M14_code_adjust_ws_levels_noop": lambda: sub(R / "java_bidi.rs", "    fn adjust_ws_levels(&mut self) {\n        if self.flags & MASK_WS == 0 {", "    fn adjust_ws_levels(&mut self) {\n        if self.flags & MASK_WS == 0 || true {"),
 "M15_code_component_split_disabled": lambda: sub(R / "font_chain.rs", "let level = |token: usize| levels.as_ref().map_or(0, |l| l[first_unit[token]]);", "let level = |token: usize| { let _ = (&levels, &first_unit, token); 0u8 };"),
 "M16_code_max_explicit_level_61": lambda: sub(R / "java_bidi.rs", "const MAX_EXPLICIT_LEVEL: u8 = 125;", "const MAX_EXPLICIT_LEVEL: u8 = 61;"),
 "M17_test_const_bold_courier_unlisted": lambda: sub(R / "font_chain.rs", 'const GDEF_BLOCKLISTED: &[&str] = &["CourierNewPSMT", "CourierNewPS-BoldMT"];', 'const GDEF_BLOCKLISTED: &[&str] = &["CourierNewPSMT"];'),
 "M18_code_no_latn_fallback": lambda: sub(R / "font_chain.rs", '.chain([*b"DFLT", *b"dflt", *b"latn"])', '.chain([*b"DFLT", *b"dflt"])'),
 "M19_code_blocklist_ignored": lambda: sub(R / "font_chain.rs", "self.gdef_glyph_classes && !self.gdef_blocklisted()", "self.gdef_glyph_classes"),
}
res = {}
for name in ["M00_unmodified"] + list(M):
    if name != "M00_unmodified": M[name]()
    diff = subprocess.run(["git", "diff", "--stat"], cwd=T, capture_output=True, text=True).stdout
    p = subprocess.run(["cargo", "test", "--release", "-j", "6", "-p", "rustuml-render", "--lib", "--", "font_chain", "java_bidi"],
                       cwd=T, capture_output=True, text=True, env={**__import__("os").environ, "CARGO_TARGET_DIR": str(LOG.parent / "target")})
    out = p.stdout + p.stderr; (LOG / f"{name}.log").write_text(diff + "\n" + out[-6000:])
    failed = re.findall(r"^    (\S+)$", out.split("failures:")[-1], re.M) if "failures:" in out else []
    summary = re.findall(r"test result: .*", out)
    res[name] = {"exit": p.returncode, "failing_tests": failed, "result": summary[-1] if summary else out[-300:], "caught": p.returncode != 0}
    print(name, res[name]["exit"], failed, flush=True)
    subprocess.run(["git", "checkout", "-q", "--", "."], cwd=T); subprocess.run(["git", "clean", "-fdq"], cwd=T)
json.dump(res, open(LOG / "drill.json", "w"), indent=1)
print("caught", sum(v["caught"] for k, v in res.items() if k != "M00_unmodified"), "/", len(M), "; unmodified exit", res["M00_unmodified"]["exit"])
