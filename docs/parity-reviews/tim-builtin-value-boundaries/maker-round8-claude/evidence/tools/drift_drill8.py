"""Round-8 drift-gate mutation drill. Usage: python3 -I drift_drill8.py <tip src> <scratch> <target> <logdir>
Each mutation is applied to a fresh copy of <tip src>; the font_chain and java_bidi lib tests must fail
(D0, unmodified, must pass)."""
import json, pathlib, re, shutil, subprocess, sys
tip, scratch, target, logs = map(pathlib.Path, sys.argv[1:5]); logs.mkdir(parents=True, exist_ok=True)
R = "crates/rustuml-render/src/"; FCU = R + "font_chain_unicode.rs"; FCC = R + "font_chain_coverage.rs"
FC = R + "font_chain.rs"; JBD = R + "java_bidi_data.rs"; SS = "tools/font-chain-reference/shaping-sources/"
PF = "tools/font-chain-reference/colossus-primary-faces/"
def sub(path, old, new):
    def f(root):
        p = root / path; s = p.read_text(); assert old in s, (path, old); p.write_text(s.replace(old, new, 1))
    return f
def append(path, data):
    def f(root):
        p = root / path; p.write_bytes(p.read_bytes() + data)
    return f
def resum(directory):
    def f(root):
        d = root / directory; lines = []
        for name in sorted(x.relative_to(d).as_posix() for x in d.rglob("*") if x.is_file() and x.name != "SHA256SUMS"):
            import hashlib; lines.append(f"{hashlib.sha256((d / name).read_bytes()).hexdigest()}  {name}")
        (d / "SHA256SUMS").write_text("\n".join(lines) + "\n")
    return f
def both(*fs):
    def f(root):
        for g in fs: g(root)
    return f
M = [
 ("D0_unmodified", lambda root: None),
 ("D1_ucd18_mn_run_1ACF_1AF0_removed", sub(FCU, "(0x1ABF, 0x1AF0),", "(0x1ABF, 0x1ACE),")),
 ("D2_hangul_group_to_indic", sub(FCU, "(18, ShaperGroup::Hangul)", "(18, ShaperGroup::Indic)")),
 ("D3_blocklist_courier_triple_changed", sub(FCU, "(588, 5078, 14418)", "(588, 5078, 14419)")),
 ("D4_courier_gdef_length_changed", sub(FCC, "gdef_length: 588,", "gdef_length: 589,")),
 ("D5_bidi_class_run_changed", sub(JBD, "(0x5C8, 0x5FF, 1)", "(0x5C8, 0x5FF, 13)")),
 ("D6_pending_verdana_bold_readded", sub(FC, "const PENDING_REFERENCE_HASH: &[&str] = &[];", 'const PENDING_REFERENCE_HASH: &[&str] = &["Verdana-Bold"];')),
 ("D7_stored_shaper_hh_edited", append(SS + "harfbuzz-15.0.0/hb-ot-shaper.hh", b" ")),
 ("D8_stored_shaper_hh_and_sums_edited", both(sub(SS + "harfbuzz-15.0.0/hb-ot-shaper.hh", "case HB_SCRIPT_HANGUL:", "case HB_SCRIPT_TELUGU2:"), resum(SS))),
 ("D9_courier_blocklist_triples_removed", both(sub(FCU, "    (588, 5078, 14418),\n", ""), sub(FCU, "    (588, 5078, 14238),\n", ""))),
 ("D10_mm_record_courier_hash_and_sums", both(sub(PF + "monkeyman-round7-font-hash.txt", "c886ca3172d8", "c886ca3172d9"), resum(PF))),
 ("D11_ubidi_icu_byte_and_sums", both(append(SS + "jdk-21.0.5-ga/ubidi.icu", b"\0"), resum(SS))),
 ("D12_bracket_pair_changed", sub(JBD, "(0x28, 0x29, true)", "(0x28, 0x5D, true)")),
 ("D13_courier_gsub_script_dropped", sub(FCC, 'gsub_scripts: &[*b"arab", *b"hebr", *b"latn"],', 'gsub_scripts: &[*b"arab", *b"hebr"],')),
]
res = {}
for name, mutate in M:
    if scratch.exists(): shutil.rmtree(scratch)
    shutil.copytree(tip, scratch, symlinks=True)
    try:
        mutate(scratch); applied = True
    except AssertionError as e:
        applied = False; print(name, "NOT APPLIED", e)
    # Fresh mtimes: copytree keeps the tip's, which cargo would take as unchanged.
    for f in (scratch / "crates/rustuml-render/src").rglob("*.rs"): f.touch()
    p = subprocess.run(["cargo", "test", "--release", "-j", "6", "-p", "rustuml-render", "--lib", "--", "font_chain", "java_bidi"],
                       cwd=scratch, capture_output=True, text=True, env={**__import__("os").environ, "CARGO_TARGET_DIR": str(target)})
    (logs / f"{name}.log").write_text("\n".join(l for l in (p.stdout + p.stderr).splitlines() if re.search(r"test |panicked|left|right|FAILED|error", l)))
    failed = re.findall(r"^test (\S+) \.\.\. FAILED", p.stdout, re.M)
    res[name] = {"applied": applied, "exit": p.returncode, "failed_tests": failed}
    print(name, "exit", p.returncode, failed)
(logs / "drill.json").write_text(json.dumps(res, indent=1))
caught = sum(1 for n, r in res.items() if n != "D0_unmodified" and r["exit"] != 0)
print("caught", caught, "/", len(res) - 1, "; D0 exit", res["D0_unmodified"]["exit"])
