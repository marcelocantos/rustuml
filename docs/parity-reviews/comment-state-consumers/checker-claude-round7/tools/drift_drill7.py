"""Checker round-7 drift-gate mutation drill (independent of the maker's drill).
Each mutation starts from a pristine copy of the tip export; runs every font_chain test in rustuml-render.
Usage: python3 -I drift_drill7.py <pristine export> <scratch export> <target dir> <out dir> <mm output.txt>"""
import hashlib, json, pathlib, re, shutil, subprocess, sys
P, S, T, O, MM = map(pathlib.Path, sys.argv[1:6]); O.mkdir(parents=True, exist_ok=True)
PD = "tools/font-chain-reference/colossus-primary-faces"; GEN = "crates/rustuml-render/src/font_chain_coverage.rs"
UNI = "crates/rustuml-render/src/font_chain_unicode.rs"; FC = "crates/rustuml-render/src/font_chain.rs"
sha = lambda b: hashlib.sha256(b).hexdigest()
def restore():
    for rel in [PD, "tools/font-chain-reference/monkeyman-round6"]:
        shutil.rmtree(S / rel); shutil.copytree(P / rel, S / rel, copy_function=shutil.copy)
    for rel in [GEN, UNI, FC]: shutil.copy(P / rel, S / rel); (S / rel).touch()
def edit(rel, fn):
    p = S / rel; old = p.read_text(); new = fn(old); assert new != old, rel; p.write_text(new)
def resum(name):  # refresh SHA256SUMS line for <name> in PRIMARY_DIR, then the recorded constant
    d = S / PD; lines = (d / "SHA256SUMS").read_text().split("\n"); h = sha((d / name).read_bytes())
    lines = [(h + "  " + name) if l.endswith("  " + name) else l for l in lines]; (d / "SHA256SUMS").write_text("\n".join(lines))
def reconst():
    new = sha((S / PD / "SHA256SUMS").read_bytes())
    edit(GEN, lambda s: re.sub(r'(PRIMARY_SHA256SUMS_SHA256: &str =\s*")[0-9a-f]{64}"', lambda m: m.group(1) + new + '"', s))
def face_block(s, const, fn):
    a = s.index(f"pub(crate) const {const}: FaceCoverage"); b = s.index("\n};", a); return s[:a] + fn(s[a:b]) + s[b:]
def run(name, expect):
    p = subprocess.run(["cargo", "test", "--release", "-j", "6", "-p", "rustuml-render", "--lib", "font_chain"], cwd=S,
                       capture_output=True, text=True, env={**__import__("os").environ, "CARGO_TARGET_DIR": str(T)})
    log = p.stdout + p.stderr; (O / f"{name}.log").write_text(log)
    failed = sorted(set(re.findall(r"^    (font_chain::tests::\S+)$", log, re.M)))
    res = re.search(r"test result: (\w+)\. (\d+) passed; (\d+) failed", log)
    first = re.search(r"panicked at [^\n]*\n([^\n]*)", log)
    row = {"mutation": name, "expect": expect, "exit": p.returncode, "result": res.group(0) if res else None,
           "failed_tests": failed, "first_assertion": (first.group(1)[:220] if first else None),
           "outcome": ("caught" if p.returncode != 0 else "passed")}
    row["as_expected"] = (row["outcome"] == expect); print(json.dumps(row)); return row
rows = []
restore(); rows.append(run("D0_unmodified", "passed"))
def own_run(name):
    def f(s):
        L = s.split("\n"); i = [k for k, l in enumerate(L) if l.startswith("P\t")][5]; x = L[i].split("\t"); x[2] = format(int(x[2], 16) + 1, "X"); L[i] = "\t".join(x); return "\n".join(L)
    edit(f"{PD}/{name}", f)
restore(); own_run("LucidaGrande.own.tsv"); rows.append(run("D1_lucida_own_run_sums_untouched", "caught"))
restore(); own_run("LucidaGrande.own.tsv"); resum("LucidaGrande.own.tsv"); rows.append(run("D2_lucida_own_run_and_sums", "caught"))
restore(); own_run("LucidaGrande.own.tsv"); resum("LucidaGrande.own.tsv"); reconst(); rows.append(run("D3_lucida_own_run_sums_constant", "caught"))
# D4: Menlo's primary-mapped table: drop its last run
def d4(s):
    m = re.search(r"pub\(crate\) const MENLO: FaceCoverage = FaceCoverage \{.*?primary_mapped: (PRIMARY_MAPPED_\d+)", s, re.S); t = m.group(1)
    a = s.index(f"const {t}: &[(u32, u32)] = &["); b = s.index("];", a); body = s[a:b].rstrip().rstrip(",").rsplit("\n", 1)[0] + "\n"
    return s[:a] + body + s[b:]
restore(); edit(GEN, d4); rows.append(run("D4_generated_menlo_primary_run_dropped", "caught"))
restore(); edit(GEN, lambda s: face_block(s, "COURIER_NEW", lambda b: b.replace("gpos_hebrew_script: true", "gpos_hebrew_script: false"))); rows.append(run("D5_generated_couriernew_hebr_fact", "caught"))
restore(); edit(f"{PD}/ArialMT.own.tsv", lambda s: s.replace("gpos_scripts=arab,cyrl,grek,hebr,latn", "gpos_scripts=arab,cyrl,grek,latn")); resum("ArialMT.own.tsv"); reconst(); rows.append(run("D6_extraction_arial_gpos_hebr_dropped_sums_constant", "caught"))
restore(); edit(GEN, lambda s: face_block(s, "VERDANA_BOLD", lambda b: b.replace("kern: false", "kern: true", 1))); rows.append(run("D7_generated_verdana_bold_kern_bit", "caught"))
def hashline(name, path, new=None, drop=False):
    def f(s):
        out = []
        for l in s.split("\n"):
            if l.endswith("  " + path):
                if drop: continue
                l = new + "  " + path
            out.append(l)
        return "\n".join(out)
    edit(f"{PD}/{name}", f)
restore(); hashline("monkeyman-hb-static.txt", "/System/Library/Fonts/Supplemental/Verdana.ttf", "0" * 64); resum("monkeyman-hb-static.txt"); reconst(); rows.append(run("D8_reference_verdana_hash_changed", "caught"))
restore(); hashline("colossus-font-files.sha256", "/System/Library/Fonts/Menlo.ttc", "f" * 64); resum("colossus-font-files.sha256"); reconst(); rows.append(run("D9_colossus_menlo_hash_changed", "caught"))
restore(); hashline("monkeyman-hb-static.txt", "/System/Library/Fonts/Helvetica.ttc", drop=True); resum("monkeyman-hb-static.txt"); reconst(); rows.append(run("D10_reference_helvetica_line_removed", "caught"))
restore(); edit(FC, lambda s: s.replace('const PENDING_REFERENCE_HASH: &[&str] = &["Verdana-Bold"];', 'const PENDING_REFERENCE_HASH: &[&str] = &["Verdana-Bold", "ArialMT"];')); rows.append(run("D11_pending_list_extended", "caught"))
restore(); edit(f"{PD}/LucidaGrande.own.tsv", lambda s: s.replace("trak=0\tSTAT=0", "trak=1\tSTAT=1")); resum("LucidaGrande.own.tsv"); reconst(); rows.append(run("D12_extraction_lucida_trak_and_stat", "caught"))
restore(); edit(GEN, lambda s: face_block(s, "ARIAL", lambda b: b.replace("notdef_gdef_class: 1", "notdef_gdef_class: 3"))); rows.append(run("D13_generated_arial_notdef_class_mark", "caught"))
# Unicode tables (not part of the font gate; checks whether any test pins them)
def d14(s):
    a = s.index("const NONSPACING_MARKS"); b = s.index("];", a); seg = s[a:b]
    seg2 = re.sub(r"\n\s*\(0x1D25B, 0x1D25C\),", "", seg); assert seg2 != seg; return s[:a] + seg2 + s[b:]
restore(); edit(UNI, d14); rows.append(run("D14_unicode18_mn_run_1D25B_removed", "unknown"))
restore(); edit(UNI, lambda s: s.replace("(18, Shaper::Hangul)", "(18, Shaper::Indic)")); rows.append(run("D15_hangul_shaper_mapped_to_indic", "unknown"))
restore(); edit(FC, lambda s: s.replace("| 0x180B..=0x180F", "| 0x180B..=0x180E", 1)); rows.append(run("D16_u180f_not_default_ignorable", "caught"))
# Positive control: the Monkey Man font-hash relay resolves Verdana Bold
mm = {l.split("  ", 1)[1]: l.split("  ", 1)[0] for l in MM.read_text().split("\n") if re.match(r"^[0-9a-f]{64}  /", l)}
vb = "/System/Library/Fonts/Supplemental/Verdana Bold.ttf"
def add_vb(s):
    L = s.split("\n"); i = max(k for k, l in enumerate(L) if l.endswith("/System/Library/Fonts/Supplemental/Verdana.ttf")); L.insert(i + 1, mm[vb] + "  " + vb); return "\n".join(L)
restore(); edit(f"{PD}/monkeyman-hb-static.txt", add_vb); resum("monkeyman-hb-static.txt"); reconst()
edit(FC, lambda s: s.replace('const PENDING_REFERENCE_HASH: &[&str] = &["Verdana-Bold"];', "const PENDING_REFERENCE_HASH: &[&str] = &[];"))
rows.append(run("D17_positive_verdana_bold_hash_from_mm_relay_pending_emptied", "passed"))
restore()
same = all((S / r).read_bytes() == (P / r).read_bytes() for r in [GEN, UNI, FC]) and \
    all((S / PD / f.name).read_bytes() == f.read_bytes() for f in (P / PD).iterdir())
summary = {"mutations": len(rows) - 1, "caught": sum(r["outcome"] == "caught" for r in rows[1:]),
           "as_expected": sum(r["as_expected"] for r in rows), "restored": same, "rows": rows}
(O / "drill.json").write_text(json.dumps(summary, indent=1)); print("RESTORED" if same else "NOT RESTORED")
