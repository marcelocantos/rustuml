"""Checker round-9 (claude) mutation drill of the per-face drift gate, the Java bidi oracle and the routing code.
Usage: python3 -I drift_drill9c.py <git-tracked 14dd5efae copy> <logdir>
Each mutation is applied to the copy; `cargo test --release -p rustuml-render --lib -- font_chain java_bidi text_render
plantuml_metrics class` runs (plus parity_perturbations for the routing mutations the lib cannot see); the result is
recorded and the copy restored with git checkout/clean. "regen" mutations are consistent forgeries: the extraction file is
edited, its SHA256SUMS line rewritten, and font_chain_coverage.rs regenerated with tools/font_chain_coverage.py + rustfmt."""
import gzip, hashlib, json, os, pathlib, re, subprocess, sys
T = pathlib.Path(sys.argv[1]); LOG = pathlib.Path(sys.argv[2]); LOG.mkdir(exist_ok=True)
R = T / "crates/rustuml-render/src"; MM = T / "tools/font-chain-reference/monkeyman-round9"; OR = T / "tools/font-chain-reference/java-bidi-oracle"
sha = lambda b: hashlib.sha256(b).hexdigest()
def sub(path, old, new, count=1):
    t = path.read_text(); assert old in t, (path, old); path.write_text(t.replace(old, new, count))
SPEC = []
for const, ps, own in [("LUCIDA_GRANDE", "LucidaGrande", "LucidaGrande.plain"), ("LUCIDA_GRANDE_BOLD", "LucidaGrande-Bold", "LucidaGrande-Bold.bold"),
    ("ARIAL", "ArialMT", "ArialMT.plain"), ("ARIAL_BOLD", "Arial-BoldMT", "Arial-BoldMT.bold"), ("ARIAL_ITALIC", "Arial-ItalicMT", "Arial-ItalicMT.italic"),
    ("ARIAL_BOLD_ITALIC", "Arial-BoldItalicMT", "Arial-BoldItalicMT.bolditalic"), ("HELVETICA", "Helvetica", "Helvetica.plain"),
    ("HELVETICA_BOLD", "Helvetica-Bold", "Helvetica-Bold.bold"), ("HELVETICA_OBLIQUE", "Helvetica-Oblique", "Helvetica-Oblique.italic"),
    ("HELVETICA_BOLD_OBLIQUE", "Helvetica-BoldOblique", "Helvetica-BoldOblique.bolditalic"), ("VERDANA", "Verdana", "Verdana.plain"),
    ("VERDANA_BOLD", "Verdana-Bold", "Verdana-Bold.bold"), ("VERDANA_ITALIC", "Verdana-Italic", "Verdana-Italic.italic"),
    ("VERDANA_BOLD_ITALIC", "Verdana-BoldItalic", "Verdana-BoldItalic.bolditalic"), ("MENLO", "Menlo-Regular", "Menlo-Regular.plain"),
    ("MENLO_BOLD", "Menlo-Bold", "Menlo-Bold.bold"), ("MENLO_ITALIC", "Menlo-Italic", "Menlo-Italic.italic"), ("MENLO_BOLD_ITALIC", "Menlo-BoldItalic", "Menlo-BoldItalic.bolditalic"),
    ("COURIER_NEW", "CourierNewPSMT", "CourierNewPSMT.plain"), ("COURIER_NEW_BOLD", "CourierNewPS-BoldMT", "CourierNewPS-BoldMT.bold"),
    ("COURIER_NEW_ITALIC", "CourierNewPS-ItalicMT", "CourierNewPS-ItalicMT.italic"), ("COURIER_NEW_BOLD_ITALIC", "CourierNewPS-BoldItalicMT", "CourierNewPS-BoldItalicMT.bolditalic")]:
    SPEC.append(f"{const}={ps}.tsv:{own}.own.tsv")
def resum_mm(*names):
    lines = (MM / "SHA256SUMS").read_text().splitlines()
    lines = [f"{sha((MM / l.split('  ', 1)[1]).read_bytes())}  {l.split('  ', 1)[1]}" if l.split("  ", 1)[1] in names else l for l in lines]
    (MM / "SHA256SUMS").write_text("\n".join(lines) + "\n")
def regen(*names):
    resum_mm(*names); d = "tools/font-chain-reference/monkeyman-round9"
    subprocess.run(["python3", "-I", "tools/font_chain_coverage.py", "crates/rustuml-render/src/font_chain_coverage.rs", d, "--primary", d, *SPEC], cwd=T, check=True, capture_output=True)
    subprocess.run(["rustfmt", "--edition", "2024", "crates/rustuml-render/src/font_chain_coverage.rs"], cwd=T, check=True)
def lengths(name, triple):
    p = MM / name; t = p.read_text(); t = re.sub(r"gdef_length=\d+\tgsub_length=\d+\tgpos_length=\d+", "gdef_length=%d\tgsub_length=%d\tgpos_length=%d" % triple, t); p.write_text(t)
def oracle_forge(edit):
    p = OR / "java-bidi-levels.txt.gz"; text = gzip.decompress(p.read_bytes()).decode(); text = edit(text)
    p.write_bytes(gzip.compress(text.encode(), 9, mtime=0))
    s = (OR / "SHA256SUMS").read_text().splitlines(); s = [f"{sha(p.read_bytes())}  java-bidi-levels.txt.gz" if l.endswith("java-bidi-levels.txt.gz") else l for l in s]
    (OR / "SHA256SUMS").write_text("\n".join(s) + "\n")
    sub(R / "java_bidi.rs", "550486611b44c6a2af7c92137e7e1847f2b747d1286e7b471f72c414b0866d5e", sha((OR / "SHA256SUMS").read_bytes()))
def drop_line(text):
    lines = text.split("\n"); i = next(k for k, l in enumerate(lines) if l.startswith("D;") and "2029" in l); del lines[i]; return "\n".join(lines)
def flip_level(text):
    lines = text.split("\n"); i = next(k for k, l in enumerate(lines) if l.startswith("D;") and "|1;0;" in l)
    a, b = lines[i].rsplit(";", 1); runs = b.split(" "); runs[-1] = str(int(runs[-1]) ^ 1); lines[i] = a + ";" + " ".join(runs); return "\n".join(lines)
def m_hash(path_fragment, remove=False):
    def f():
        p = MM / "font-hashes.txt"; lines = p.read_text().split("\n")
        out = []
        for l in lines:
            if l.endswith(path_fragment):
                if remove: continue
                h, rest = l.split("  ", 1); l = h[:-1] + ("0" if h[-1] != "0" else "1") + "  " + rest
            out.append(l)
        p.write_text("\n".join(out)); regen("font-hashes.txt")
    return f
def cov(variant, orig, table):
    return lambda: sub(R / "font_chain.rs", f"Self::{variant} => &coverage::{orig},", f"Self::{variant} => &coverage::{table},")
PERT = {"M00", "M15", "M16", "M21"}
M = {
 "M01_extraction_verdana_italic_chain_run_dropped_sums_untouched": lambda: sub(MM / "Verdana-Italic.tsv", "C\tA0\t36F\n", ""),
 "M02_extraction_courier_italic_lengths_blocklisted_regen": lambda: (lengths("CourierNewPS-ItalicMT.italic.own.tsv", (588, 5078, 14418)), regen("CourierNewPS-ItalicMT.italic.own.tsv")),
 "M03_extraction_courier_bolditalic_lengths_blocklisted_regen": lambda: (lengths("CourierNewPS-BoldItalicMT.bolditalic.own.tsv", (588, 5078, 14238)), regen("CourierNewPS-BoldItalicMT.bolditalic.own.tsv")),
 "M04_extraction_lucida_italic_real_italic_face_regen": lambda: (sub(MM / "LucidaGrande.italic.own.tsv", "#java_face\tLucidaGrande\t2\tLucidaGrande", "#java_face\tLucidaGrande\t2\tLucidaGrande-Italic"), regen("LucidaGrande.italic.own.tsv")),
 "M05_extraction_menlo_italic_notdef_regen": lambda: (sub(MM / "Menlo-Italic.tsv", "#notdef_advance_units\t1233.0", "#notdef_advance_units\t1234.0"), sub(MM / "Menlo-Italic.italic.own.tsv", "#notdef_advance_units\t1233.0", "#notdef_advance_units\t1234.0"), regen("Menlo-Italic.tsv", "Menlo-Italic.italic.own.tsv")),
 "M06_mm_font_hash_courier_bold_changed_regen": m_hash("Courier New Bold.ttf"),
 "M07_mm_font_hash_courier_italic_changed_regen": m_hash("Courier New Italic.ttf"),
 "M08_mm_font_hash_helvetica_removed_regen": m_hash("Helvetica.ttc", remove=True),
 "M09_code_arial_italic_uses_plain_table": cov("ArialItalic", "ARIAL_ITALIC", "ARIAL"),
 "M10_code_courier_italic_uses_plain_table": cov("CourierNewItalic", "COURIER_NEW_ITALIC", "COURIER_NEW"),
 "M11_code_menlo_bolditalic_uses_bold_table": cov("MenloBoldItalic", "MENLO_BOLD_ITALIC", "MENLO_BOLD"),
 "M12_code_helvetica_boldoblique_uses_bold_table": cov("HelveticaBoldOblique", "HELVETICA_BOLD_OBLIQUE", "HELVETICA_BOLD"),
 "M13_code_verdana_bolditalic_uses_bold_table": cov("VerdanaBoldItalic", "VERDANA_BOLD_ITALIC", "VERDANA_BOLD"),
 "M14_code_sansserif_italic_maps_to_bold": lambda: sub(R / "font_chain.rs", "(ChainFamily::LucidaGrande, false, _) => LucidaGrande,", "(ChainFamily::LucidaGrande, false, false) => LucidaGrande,\n            (ChainFamily::LucidaGrande, false, true) => LucidaGrandeBold,"),
 "M15_code_arial_italic_routed_plain": lambda: sub(R / "text_render.rs", "ChainFace::for_style(ChainFamily::Arial, bold, italic)", "ChainFace::for_style(ChainFamily::Arial, bold, false)"),
 "M16_code_courier_width_ignores_italic": lambda: sub(R / "plantuml_metrics.rs", "let face = ChainFace::for_style(ChainFamily::CourierNew, bold, italic);", "let face = ChainFace::for_style(ChainFamily::CourierNew, bold, { let _ = italic; false });"),
 "M17_code_run_levels_per_paragraph": lambda: sub(R / "java_bidi.rs", "Direction::Ltr | Direction::Rtl => vec![self.para_level; self.levels.len()],", "Direction::Ltr | Direction::Rtl => self.levels.clone(),"),
 "M18_code_run_levels_mixed_single_run": lambda: sub(R / "java_bidi.rs", "Direction::Mixed => self.levels.clone(),", "Direction::Mixed => vec![self.para_level; self.levels.len()],"),
 "M19_oracle_line_dropped_sums_and_constant_forged": lambda: oracle_forge(drop_line),
 "M20_oracle_java_run_level_flipped_sums_and_constant_forged": lambda: oracle_forge(flip_level),
 "M21_code_class_link_label_measured_sans13": lambda: subprocess.run(["perl", "-0pi", "-e", r"s/let \(font_family, font_size\) =\s*relationship_label_font\(&relationship_arrow_style\(diagram, relationship\)\);/let (font_family, font_size) = (\"sans-serif\".to_string(), RELATIONSHIP_LABEL_FONT_SIZE);/g", str(R / "class.rs")], check=True),
}
env = {**os.environ, "CARGO_TARGET_DIR": str(LOG.parent / "target-drill"), "PLANTUML_STDLIB": str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib")}
res = {}
for name in ["M00_unmodified"] + list(M):
    if name != "M00_unmodified": M[name]()
    diff = subprocess.run(["git", "diff", "--stat"], cwd=T, capture_output=True, text=True).stdout
    p = subprocess.run(["cargo", "test", "--release", "-j", "6", "-p", "rustuml-render", "--lib", "--", "font_chain", "java_bidi", "text_render", "plantuml_metrics", "class"],
                       cwd=T, capture_output=True, text=True, env=env)
    out = p.stdout + p.stderr
    failed = re.findall(r"^    (\S+)$", out.split("failures:")[-1], re.M) if "failures:" in out else []
    summary = re.findall(r"test result: .*", out)
    r = {"lib_exit": p.returncode, "lib_failing_tests": failed, "lib_result": summary[-1] if summary else out[-400:]}
    if name[:3] in PERT:
        q = subprocess.run(["cargo", "test", "--release", "-j", "6", "-p", "rustuml-oracle", "--test", "parity_perturbations", "--", "held_out_perturbations_match_java_without_oracle_layout"],
                           cwd=T, capture_output=True, text=True, env=env)
        qo = q.stdout + q.stderr
        r["perturbations_exit"] = q.returncode
        r["perturbation_failures"] = len(set(re.findall(r"^  (test-diagrams/perturbations/\S+\.puml):", qo, re.M)))
        out += "\n==== parity_perturbations ====\n" + qo[-4000:]
    (LOG / f"{name}.log").write_text(diff + "\n" + out[-8000:])
    r["caught_by_lib"] = p.returncode != 0
    res[name] = r
    print(name, r, flush=True)
    subprocess.run(["git", "checkout", "-q", "--", "."], cwd=T); subprocess.run(["git", "clean", "-fdq"], cwd=T)
json.dump(res, open(LOG / "drill.json", "w"), indent=1)
