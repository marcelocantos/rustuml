"""Write the two round-5 review records and copy the shared evidence byte-identically into each mechanism's
checker-claude-round5/ directory with SHA256SUMS.json. Usage: python3 -I write_reviews.py <repo> <evidence_dir>"""
import hashlib, json, pathlib, shutil, sys
REPO = pathlib.Path(sys.argv[1]); EV = pathlib.Path(sys.argv[2])
pairs = json.load(open(EV / "pairs.json"))
H = {"base": "fbb2e59edc4beaccda72c30fe19e8a38ca8e1632", "base_code": "9834a719455308120c3289b59a260567f141b724",
     "merge": "d6f63a8aa912d9652a6986de427da10209c73ec6", "r4tip": "7ec2e5af8a37ace145b37b78951ce217c0bab51a",
     "tip": "8dda18b1bf819f379c0c13d1cadd158400eed147", "accounts": "f5442f4e5ef7c6f440226a25281ae9bf2339039c",
     "origin": "f3ebc92f3468b039ea2093fd54f36a1bcb9f6a81", "names": "3ebcb763ebcdbaa3286d2fdf4e320ee0ae699858",
     "glyph": "4a16eaffd159e886fec31db0af3517b2eff6f25f", "decode": "673548fb95a4520ca4af669d09cd64c9abcd153e",
     "yaml_add": "a4e895aaf5f4cc826fe2b3fc19e54a2ec1bfc989", "yaml": "a18a76980321ba0aef86dee963d426af06d88d89",
     "m5a": "02fa4159febdfb43a0627417d6d654195bf79bde", "pt3_add": "d80bee422ab3c7c7a4d1b2a65218990f6b554e95",
     "pt3": "ce1e52e30956dc4593dfcc915a34be908cf66029", "clippy": "5149e8ecbdc149bc1b7bf4cb4043d38626154d72",
     "r4review": "6caf4984570cf5ed87f21ac32572bc47ae93363c"}
JAVA = "71806a23780b04a5ccde2f8ceb5121edad5eb711"; JAR = "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053"
SUB = "checker-claude-round5"; CACHE = "~/Library/Caches/rustuml-shared-svg-round5-claude-checker"
COMMANDS = [
    f"git switch -c review/shared-svg-round5-claude {H['tip']}; git -c protocol.file.allow=always submodule update --init --reference /Users/marcelo/work/github.com/marcelocantos/rustuml/test-diagrams/golden test-diagrams/golden  # 9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a",
    f"{CACHE}/build.sh  # git archive of fbb2e59ed, d6f63a8aa, 8dda18b1b; CARGO_TARGET_DIR private; cargo build --release -j 6 -p rustuml",
    f"python3 -I tools/batch.py {CACHE}/work <batch dir> <label> bin/rustuml-fbb2e59ed=base bin/rustuml-d6f63a8aa=merge bin/rustuml-8dda18b1b=tip [bin/rustuml-tip-ev=ev]  # BH BG BF BE BC BB AW AU AR",
    "python3 -I tools/sweep.py <bin> <out> <repo> work/cwd_map.json test-diagrams/golden test-diagrams/perturbations  # 16,508 sources x 3 trees (+ tip-ev); checker-cmp; tools/cohorts.py",
    "bash tools/gates.sh <tip export> tip ...; base/merge exports: golden_no_oracle, parity_perturbations, harness_guards; tools/gatelists.py",
    "python3 -I tools/c5a_sources.py; tools/predict.py (Rust predictions before Java); tools/stage.py -> parent relay BH on Monkey Man",
    "font-chain: swiftc FontChainExtract.swift (maker copy) and the CFLocaleCopyISOLanguageCodes variant on colossus; parent ran font-chain/request/run.sh on Monkey Man; host-coverage-diff.json",
    "python3 -I tools/register.py <repo> pairs.json BG=... BF=... BH=...; python3 -I tools/write_reviews.py <repo> evidence",
]
REVIEWER = {"task_id": "t3-2bb601af-independent-shared-svg-round5-checker-claude",
            "model": "claude-opus-5-5 / Claude Code in T3 Code; no children; did not implement the candidate; no Java run locally (Java references BG and BH run by the parent on Monkey Man)"}
def perts(mech):
    out = []
    for r in pairs[mech]:
        if not r["registered"]: continue
        out.append({k: r[k] for k in ["name", "batch", "reference_batch", "axes", "intended_valid", "java_exit", "source", "golden", "cwd",
                                       "source_sha256", "golden_sha256"]} | {
            "base_fbb2e59ed": f"{r['base_fbb2e59ed_cli_exit']}/{r['base_fbb2e59ed_strict_compare']}/{r['base_fbb2e59ed_difference_count']}/comments={r['base_fbb2e59ed_comment_list_exact']}",
            "merge_d6f63a8aa": f"{r['merge_d6f63a8aa_cli_exit']}/{r['merge_d6f63a8aa_strict_compare']}/{r['merge_d6f63a8aa_difference_count']}/comments={r['merge_d6f63a8aa_comment_list_exact']}",
            "tip_8dda18b1b": f"{r['tip_8dda18b1b_cli_exit']}/{r['tip_8dda18b1b_strict_compare']}/{r['tip_8dda18b1b_difference_count']}/comments={r['tip_8dda18b1b_comment_list_exact']}",
            "result": r["result"]})
    return out
evidence_only = lambda mech: [{k: r.get(k) for k in ["name", "batch", "java_exit", "java_crash_text", "reason_not_registered", "tip_outcome_vs_java", "source_sha256"]}
                              for r in pairs[mech] if not r["registered"]]
common = {"schema_version": 1, "reviewer": REVIEWER, "java_revision": JAVA, "jar_sha256": JAR,
          "base_revision": H["base"], "base_code_identical_to": H["base_code"], "merge": H["merge"], "merge_code_identical_to": H["r4tip"],
          "tested_revision": H["tip"], "answers_rejection": H["r4review"], "commands": COMMANDS,
          "no_production_changes": True, "no_shipping": True,
          "account_chronology": {"accounts (docs only)": H["accounts"], "outer origin draft": H["origin"], "note names draft": H["names"],
                                 "missing glyph + width decoding draft": H["glyph"], "decode-site escape draft": H["decode"],
                                 "YAML addendum -> draft": [H["yaml_add"], H["yaml"]], "m5a request (before any Java result)": H["m5a"],
                                 "pass-three addendum -> draft": [H["pt3_add"], H["pt3"]], "clippy only": H["clippy"],
                                 "every account and addendum precedes its draft in linear ancestry; account commits are docs-only": True},
          "frozen_items": "unchanged fbb2e59ed..8dda18b1b: crates/rustuml-oracle (comparator slack GEOM_EPS and canvas +-1, syntax markers, park list, error census, guard allow-lists, strict-XML list), decimal_literal_baseline.txt, no_oracle_baseline.txt, docs/reference-bootstrap.md; round 5 touches no docs or test-diagrams outside its maker-round5-claude directories",
          "evidence": {"dir": f"{SUB} (byte-identical copy in docs/parity-reviews/comment-state-consumers/ and docs/parity-reviews/tim-builtin-value-boundaries/; SHA256SUMS.json)",
                       "measurements": f"{SUB}/measurements.json", "batches": f"{SUB}/batches/batch-*.json, {SUB}/batch-summary.json",
                       "cohorts": f"{SUB}/cohorts.json", "pairs": f"{SUB}/pairs.json", "gates": f"{SUB}/gates",
                       "font_chain": f"{SUB}/font-chain (host-coverage-diff.json, monkeyman/, request/, jdk-native/ copies of JDK 21.0.5 GA AWTFont.m, CoreTextSupport.m, CCharToGlyphMapper.m, bundled hb-unicode.hh)",
                       "canvas": f"{SUB}/canvas (evidence-build.diff, changed-output lists)", "references": f"{SUB}/references/bh (c5a relay BH), references/bg (m5a results, maker predictions)",
                       "probes": f"{SUB}/probes (c5a sources, yaml-minimal)", "tools": f"{SUB}/tools"},
          "do_not_regress_check": "BB/BC/AW/AU/AR/BE outcomes identical merge->tip (0 lost, 0 gained except BE 5 named by review-5): note paint order and border points (BE bp_*/no_*), Graphviz stderr discard (sweep stderr only differs by panic thread ids), node creation order, leaf corners and nested-cluster x (BB), surrogate serializer rule, %intval, bracket declarations, strokes, X[H], seed and __theme (AW/AU/AR), %strlen/%substr/%upper (perturbation dirs: 0 new failures vs merge)",
          "counts": {"BH_c5a_held_out": "99 cases, 96 Java exit 0; strict+comments base/merge/tip 3/14/67 (strict only 12/14/67); 0 lost vs base or merge",
                     "BG_m5a_held_out": "53 cases, 52 Java exit 0; 0/8/41 (strict only 12/8/41); 0 lost; maker predictions equal tip output on 53/53",
                     "BF": "46 cases; 1/17/32; 0 lost", "BE": "0/17/22", "BC": "0/18/18", "BB": "2/25/25", "AW": "6/30/30", "AU": "6/21/21", "AR": "6/17/17",
                     "goldens": "12,550 sources, 11,351 Java-valid: strict 11181/11181/11181; tip exit 0 x12474, 1 x76; 0 panics, 0 timeouts",
                     "sweep_all": "16,508 sources: strict 12911/13547/13555; merge->tip 8 gained, 0 lost, 0 lost rejections, 0 new valid-input errors, 0 new panics/timeouts, 0 comments lost, 298 outputs changed, 4 stderr changed (thread ids of 4 pre-existing panics)",
                     "GET_goldens": "act_skinparam2_shadowing_true, act_skinparam_shadowing, edge_whitespace_multiple_blank_lines: MATCH at base, DIFF at merge and tip (raw-reader contract; not round 5)",
                     "gates": "no-oracle 11181/11251 base and tip (failure-name diff: -3 GET goldens, +3 gained); golden_pairs 10497/12550; parity_perturbations 0 new vs merge, 8 fixed; harness_guards categories and decimal ratchet identical to merge; T14.6.7 196/98/98 unchanged; lib 1584/1584; pipe_mode 17/17"}}
CANVAS = ("Canvas rounding (f3ebc92f3 exposes it): Java sets the SVG size through SvgGraphics.ensureVisible (SvgGraphics.java:142-147), maxX/maxY = (int)(x + 1) of the image dimension. "
          "The outer image path rounds with ceil (state.rs:5097-5098, build of the image-path SVG), although state.rs already defines svg_ensure_visible_dimension for Java's rule. The two agree for fractional extents and differ by 1 px on integral ones, "
          "which the exact LimitFinder bounds now produce. checker_history_font_state_channel (end ellipse cy 395.0, extent 419.0) lost Java's 420 height for 419; held-out BH no6_integer_canvas has Java width 108, tip 107. "
          "A private evidence build with only that line changed (canvas/evidence-build.diff) gains Java's exact canvas on 98 more sweep outputs (12,845 -> 12,943 exact viewBox of 16,508), loses none, and restores both cases. "
          "Every such case still passes the strict tier because the frozen comparator allows +-1 on the canvas, so no strict match was lost. A second, opposite +1 width exists on the root/flat path (BH pt6_note_outer_anchor_later_inner 172 vs 171, rp6_root_and_group 260 vs 259) and is not touched by the evidence build.")
cs_findings = [
    "Outer image origin (f3ebc92f3): confirmed. Held-out BG no5_left_note_renamed, no5_left_note_start_top, no5_left_note_depth2_region, no5_right_note_nested_image, no5_left_note_multiline_renamed and BH no6_left_depth2, no6_right_long, no6_left_and_start, no6_left_right_both, no6_left_multiline_depth1, no6_left_leaf_beside_image match whole-output; BE no_outer_note_left_of_image, no_outer_two_notes_on_image and BF st_np_outer_note_left_multiline now match. SvekResult.calculateDimension (6 - LimitFinder min, max - min + 15) and LimitFinder.drawUPath/drawRectangle/drawEllipse were checked against the Java source.",
    CANVAS,
    "Notes above/below and floating notes keep the flat layout and are residual as declared: BG no5_note_top_image, no5_note_bottom_image, BH no6_top_depth2, no6_floating, rp6_named_floating_in_group, BF st_np_outer_note_top_of_image, st_np_outer_floating_with_image fail identically at merge and tip.",
    "Two notes on one side of an image (BG no5_three_notes_image, BH no6_two_left_same, BF st_np_three_notes_one_cluster): Java draws the first such note as a real link (comment 'link GMN2 to Mint', all geometry shifted 42.06 px); Rust has no port of that mechanism. Pre-existing (merge fails with the same diff count); not caused by this packet, but it is an axis the account listed.",
    "Root-path note names (3ebcb763e): confirmed. BG rp5_root_note_depth2, rp5_two_groups_sides, BH rp6_depth3 (depth 3), rp6_duplicate_text (duplicate note text), rp6_root_and_group match; BH rp6_group_note_outer_anchor now has Java's Toll.GMN2 and fails only on a pre-existing 3 px width.",
    "Pass-three note anchors (d80bee422 -> ce1e52e30): confirmed. Java registers note-on-entity for ParserPass.THREE only in StateDiagramFactory (97-100), so no other family applies. BH pt6_note_before_nested_link, pt6_note_before_root_link_image, pt6_note_outer_anchor_later_inner, pt6_note_before_concurrent, pt6_two_notes_before and BF st_np_outer_note_left_on_leaf_with_image, st_np_note_before_target match.",
    "Note on a link in an image diagram (BH no6_note_on_link_image): Java keeps the image path (no 'Mill' cluster), Rust takes the cluster path. Pre-existing (merge identical); outside the declared routing residue, which names only top/bottom notes and on-leaf notes.",
    "Routing guard addendum (f6b7f66f0): structural (composites.is_empty()), not fixture-keyed; root-path diagrams with notes stay on the root cluster path (BE no_root_cluster_note, no_root_two_clusters_notes now match).",
    "Sweep canvas census: 298 outputs changed merge -> tip; 176 now have Java's exact viewBox (the maker counted 154 under its own definition), 1 lost it (checker_history_font_state_channel, cause above), 119 neither, 2 unchanged-exact.",
]
tim_findings = [
    "REJECT. YAML literal escapes (a4e895aaf -> a18a76980) cause a fresh held-out whole-output failure: BH dc6_yaml_nested (Java exit 0). Java's YAML parser keeps escapes literal (YamlLine.unquote 189-198, as the addendum says), but TextBlockJson.getTextBlock (342) then builds every key and value through Display.getWithNewlines (Display.java 284-305), which turns '\\\\' into '\\', '\\t' into a tab and '\\n'/'\\r'/'\\l' into line breaks and leaves other pairs ('\\u', '\\U', '\\\"') literal. For inner: \"a\\\\b\" Java draws 'a\\b' (textLength 23.8848); merge drew the same; the tip draws 'a\\\\b' (31.2266), and every other difference of the output (canvas 325 vs 317, box widths, the list node's x) follows from that element. Rust-only corroboration probes/yaml-minimal: base and merge 23.8848, tip 31.2266.",
    "The same Display rule explains the pre-existing BH failures dc6_yaml_plain, dc6_yaml_key_newline, dc6_yaml_apostrophe, dc6_yaml_comment, dc6_yaml_hash_in_quotes, dc6_yaml_single, dc6_yaml_block (Java splits at \\n and expands \\t; Rust draws one line). The YAML literal rule itself is confirmed where Display keeps the pair: BG dc5_yaml_key_escape, dc5_yaml_list_escape, BF cr_yaml_u_escape, BH dc6_yaml_escaped_quote, dc6_yaml_trailing_backslash, dc6_yaml_colon_key, dc6_yaml_listonly match. Flow-sequence residue (BG dc5_yaml_flow_escape) confirmed: YamlLine.toList makes the next character literal.",
    "Missing glyph (4a16eaffd), reference-host coverage: on Monkey Man (Java's host) 76 of 93 held-out BG/BH width probes match whole-output; the 17 failures are 7 Monospaced family-name/NBSP cases whose widths match, 5 declared mapped-advance residue, 2 default-ignorable residue and the 3 nonspacing marks below. Out of sample the model matches unmapped BMP scalars in new blocks (U+0870, U+1C80, U+2E5E, U+1AB0, U+0D80, U+A69E), BMP default-ignorables in simple text (missing) and complex text (hidden: U+2065, U+FFF0), supplementary CJK Ext-G/H, Adlam (cascade lacks it), tags-block and VS-supplement unassigned (hidden), Egyptian format controls (Cf, missing), combining/RTL/Thai complex text with unassigned scalars, sizes 9/11/23, bold, Arial, Verdana, Helvetica, Courier New, and sequence/activity/state/mindmap renderers.",
    "Missing glyph, falsified on held-out data: an unmapped nonspacing mark (gc=Mn) in complex text has zero advance in Java, not the primary .notdef. BH w6_sans_180f_complex (U+180F, Java 0, tip 10.5), w6_sans_1d17b (U+1D17B, Java 0, tip 10.5), w6_sans_16fe4 (U+16FE4, Java 0, tip 10.5). The maker's default-ignorable list equals HarfBuzz 8.2.2 bundled in JDK 21.0.5 exactly (hb-unicode.hh is_default_ignorable 167-197: 180B..180E, 1D173..1D17A), so the zero comes from another rule, consistent with HarfBuzz giving mark-class glyph props from the Unicode category and zeroing mark advances. These fail at base and merge too (different wrong values), so they are not regressions, but the account's 'Missing -> .notdef' is not Java's rule for marks. Not yet traced to a HarfBuzz source line: next obligation.",
    "Missing glyph, host dependence (font-chain/host-coverage-diff.json): Monkey Man and the maker's Mac run the same macOS 27.2 build 26B123, yet every per-face TSV differs. The cause is one asset family. On Monkey Man the PingFang slots of the cascade resolve to a downloaded MobileAsset (/System/Library/AssetsV2/com_apple_MobileAsset_Font8/.../PingFang.ttc); on colossus they resolve to the reserved UI copy (FontServices.framework/Resources/Reserved/PingFangUI.ttc). Identical across all 13 faces: 37 BMP scalars the simple path maps only on Monkey Man (U+31BC-31BF, 327E, F817-F81D, F828-F83B, F83F, F880-F881, FA2E-FA2F), 188 complex-path scalars mapped only on colossus (U+31D0-31E5 CJK strokes, 117 CJK Ext-B and 49 CJK Ext-C..H scalars), 31 mapped only on Monkey Man (U+31B8-31BF, 3248-324F, FE20-FE23 except Courier/Menlo/Arial, 1F101-1F10A, 1F10C). Times New Roman differs only in the 37 simple-path scalars. The table generated on colossus is therefore wrong on the reference host for those 256 scalars. Notdef advances are identical on both hosts (font-chain/monkeyman/notdef.txt).",
    "Missing glyph, user fonts: the extraction tool deliberately treats a user font (~/Library/Fonts) as missing, but Java uses user fonts. Both hosts have MesloLGS NF and Hack in ~/Library/Fonts (census). On both, CoreText substitutes MesloLGS NF for U+F8B7..F8FE and Hack for U+20B7 (font-chain/monkeyman/whofb-lucida.txt, colossus-whofb-*.txt), and the July Java Times New Roman table (times_new_roman_unicode.rs) gives exactly those scalars real advances. That is 46 of the 52 'Java advance, model missing' disagreements the maker attributed to 'fonts changed by later OS updates'; U+F880 is the downloaded LiHeiPro asset on Monkey Man. The model calls these Missing (0.75 em in SansSerif) where Java substitutes a glyph.",
    "Missing glyph, JDK fidelity: CFont.getCascadeList (AWTFont.m 547-567, JDK 21.0.5 GA) passes CFLocaleCopyISOLanguageCodes(), not nil; the checker re-ran the tool both ways on colossus for LucidaGrande and TimesNewRomanPSMT: slot order differs, coverage is identical. The simple path's substitution is JRSFontCreateFallbackFontForCharacters (CoreTextSupport.m CTS_GetGlyphsAsIntsForCharacters), not CTFontCreateForString; 'LastResort counts as missing' is supported by measurement (TNR table, BG/BH unassigned BMP probes), not traced. Simple-path substitution itself is confirmed: BH w6_sans_211a and w6_sans_22ad have Java advance 10.8897 (substituted glyph), not .notdef; they and w6_sans_a8fb, BG w5_sans_a500 fail only through the declared mapped-advance residue.",
    "Missing glyph constants: the per-face .notdef advances are font metrics, measured by CTFontGetAdvancesForGlyphs on glyph 0 on both hosts: Lucida Grande 1536, Lucida Grande Bold 1536, Arial 1536, Helvetica 1298, Helvetica Bold 1479, Verdana 2048, Menlo 1233 (= MONO_WIDTH_PER_SIZE 0.60205078125 x 2048), Courier New 1229 (= COURIER_NEW_WIDTH_PER_SIZE), Times New Roman 1593. The generated table carries a provenance header; the maker's TSVs regenerate byte-identically on colossus; no new 3+ decimal literal (ratchet identical).",
    "Residue observed out of sample (pre-existing, not caused): emoji tag flag (BH w6_sans_tag_flag: Java 19.0 for the whole sequence, tip 103: supplementary default-ignorables that the cascade maps are not hidden; the account declares this only for BMP); Times New Roman unassigned default-ignorable (BH w6_times_e0080: Java hides U+E0080, the Java Times table lists only defined scalars); Hebrew mapped advance (BH w6_sans_hebrew_missing); Monospaced 'monospace' family name and Java's NBSP in mono/courier text (BG w5_mono_*, BF w_courier_pua_b, w_monospaced_pua_b, BH w6_mono_*): widths match.",
    "Width-token decoding (4a16eaffd): confirmed. BG w5_{helvetica,verdana,couriernew,arial,times}_{10fffd,10f800}, w5_arial_10f7ff, w5_helvetica_bold_10fffd and BF w_arial_pua_b, w_times_pua_b match; controls unchanged. BF w_courier_pua_b and w_monospaced_pua_b, precommitted as fixed, now have Java's width but still fail on the pre-existing NBSP/family-name difference.",
    "Decode-site escaping for JSON and TIM JSON (673548fb9): confirmed. BG dc5_json_nested, dc5_tim_json_member, dc5_tim_json_array, BF cr_json_u_escape, BH dc6_json_control match; sweep: no golden YAML/JSON output changed merge -> tip.",
    "The '?'-in-comment rule: keep it. Java's successful outputs write the unit in a comment as '?': BG cm_unit_class_code '<!--class Q?-->', cm_unit_package_code '<!--cluster P?-->', BH cm6_class_bare and cm6_class_low (low surrogate) '<!--class Q?-->'. The maker's observability claim is false: Java does not crash on the unit in data-qualified-name, because UGroup.put applies fix (UGroup.java 69-72: NON_WORD -> '.'), giving 'Q.' and 'P..C'. Rust never reaches the rule: its code grammar drops the unit (comment 'class Q', name 'Q') and drops the package, state and object entities; pre-existing at base. Classification of the intent disagreements: cm_unit_class_code, cm_unit_package_code, cm6_class_bare Java-valid; cm6_component_code, cm6_state_code, cm6_usecase_code Java syntax errors (exit 200, 'Assumed diagram type: sequence', line 2) that Rust renders (lost rejections, pre-existing, T14.17/T14.21); cm_unit_link_code a Java export crash (Invalid UTF-16 surrogate d83d d83d) that Rust renders normally (T14.20); cm6_object_code Java-valid, Rust output nearly empty (pre-existing).",
    CANVAS,
]
cs_obligations = [
    "Use Java's (int)(x + 1) (svg_ensure_visible_dimension, SvgGraphics.ensureVisible) for the outer image path canvas (state.rs:5097-5098); acceptance: BH no6_integer_canvas and checker_history_font_state_channel exact canvas, the 98-output sweep gain of the evidence build, 0 lost exact canvases, plus a fresh integral-extent perturbation. Trace the opposite +1 on the root/flat path (BH pt6_note_outer_anchor_later_inner, rp6_root_and_group) separately.",
    "Port Java's second same-side note mechanism (the first note on a side drawn as a link, 'link GMN2 to Mint'): BG no5_three_notes_image, BH no6_two_left_same, BF st_np_three_notes_one_cluster, plus a renamed three-note variant.",
    "Route a note on a link in an image diagram as Java does (BH no6_note_on_link_image).",
    "Above/below and floating outer notes stay residual until ported; any draft names BG no5_note_top_image, no5_note_bottom_image, BH no6_top_depth2, no6_floating, rp6_named_floating_in_group.",
]
tim_obligations = [
    "Blocking: port Display.getWithNewlines for YAML/JSON keys and values (TextBlockJson.getTextBlock) on top of YamlLine's literal parse: '\\\\' -> '\\', '\\t' -> tab, '\\n'/'\\r'/'\\l' -> line break, other pairs literal. Acceptance: BH dc6_yaml_nested, dc6_yaml_plain, dc6_yaml_key_newline, dc6_yaml_apostrophe, dc6_yaml_comment, dc6_yaml_hash_in_quotes, dc6_yaml_single, dc6_yaml_block and the BG/BF/BH YAML pairs that already match, plus fresh renamed variants; JSON values must be checked too, since the same TextBlockJson path receives decoded JSON strings.",
    "Blocking: trace Java's zero advance for an unmapped nonspacing mark in complex text to a HarfBuzz 8.2.2 source line (mark glyph props from the Unicode general category and mark-advance zeroing) and port it; acceptance BH w6_sans_180f_complex, w6_sans_1d17b, w6_sans_16fe4 plus fresh Mn/Me scalars in other blocks and in simple text (where Java keeps .notdef, BH w6_sans_1ab0).",
    "Blocking: make the coverage table a declared environment input of the reference host. Generate it on Monkey Man (the Java host), including user fonts and downloaded MobileAsset fonts as Java's CoreText sees them, or validate the committed table against a Monkey Man extraction with a hash gate; record host, build and the font-asset census in the table header. Acceptance: per-face TSV equality with the reference host, and Java probes for at least the PingFang-asset scalars (e.g. U+31BC, U+F817, U+31D0, U+FE20, U+1F101) and the user-font scalars (U+F8B7, U+20B7) in simple and complex text.",
    "Align the extraction tool's cascade call with JDK 21.0.5 (CFLocaleCopyISOLanguageCodes()) and its simple-path fallback with JRSFontCreateFallbackFontForCharacters, or record measured equivalence on the reference host.",
    "Supplementary default-ignorables mapped by a cascade font (emoji tag sequences, BH w6_sans_tag_flag) and Times New Roman unassigned default-ignorables (BH w6_times_e0080) need a declared rule or a port.",
    "Rust code grammars must keep an unpaired unit in entity codes (class/package/object Java-valid; component/state/usecase Java syntax errors) before the '?' comment rule can be exercised; cm_unit_* and cm6_* pairs gate it.",
]
def write(mech, number, account, impl, other, scope, verdict_text, findings, obligations, held_out):
    d = REPO / "docs/parity-reviews" / mech
    rec = common | {"mechanism": mech, "account_commit": account, "implementation_commit": impl, "other_commits_reviewed": other,
                    "scope": scope, "verdict": "rejected", "packet_verdict": verdict_text, "per_mechanism_verdicts": held_out["verdicts"],
                    "findings": findings, "attributed_regressions": held_out["regressions"], "held_out_failures": held_out["failures"],
                    "bounded_next_obligations": obligations, "staged_java_requests_answered": {
                        "BH": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round5-c5a/", "reference_results_sha256": "6c78b9e8c64738b57313f7df6043a099a3d538e6c75a6c26744aa8fd3cf2cb55",
                               "manifest_sha256": "54000fedd4d74a42dc4a5b912cf31a5403f06e104cb9206d3c2bb071496ff504", "bundle_sha256": "805e4671b500c05a820b11beaf51af96d7e6cd137f1da835d0a4fabc9ea09afd",
                               "rust_predictions_sha256": "1e6882f8139468fa8d75a3270d5b1e4d39dab655826b12166536cad1f066c3e9"},
                        "monkeyman_font_chain": {"path": "/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-round5-monkeyman-fontchain/out/", "SHA256SUMS_sha256": "810633ca2bf2d76acf2d03e35023bd765037cd17c4d4fb5a5615bae801532f5d",
                                                 "request_sha256": "077b900b1cd6140ed5ffa69c5a21b7afeac7460554d46bca000eaac90b70a89b"}},
                    "evidence_only_cases": evidence_only(mech), "perturbations": perts(mech)}
    (d / f"review-{number}.json").write_text(json.dumps(rec, indent=1, ensure_ascii=False) + "\n")
    tgt = d / SUB
    if tgt.exists(): shutil.rmtree(tgt)
    shutil.copytree(EV, tgt)
    sums = {str(p.relative_to(tgt)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(tgt.rglob("*")) if p.is_file()}
    (tgt / "SHA256SUMS.json").write_text(json.dumps(sums, indent=1) + "\n")
    print(mech, number, len(rec["perturbations"]), "pairs,", sum(p["result"] == "pass" for p in rec["perturbations"]), "pass,", len(sums), "evidence files")
write("comment-state-consumers", 6, H["accounts"], H["pt3"],
      {"outer_origin_draft": H["origin"], "note_names_draft": H["names"], "pass_three_addendum": H["pt3_add"], "pass_three_draft": H["pt3"], "routing_guard_addendum_reviews": "f6b7f66f006108c8041bb5923d70a7d327f28053"},
      "round-5 state obligations: outer image origin and size (SvekResult/LimitFinder), root-path note quark names, pass-three note anchors, routing-guard addendum",
      "Packet 8dda18b1b rejected as a whole: the tim-builtin-value-boundaries YAML draft a18a76980 causes held-out BH dc6_yaml_nested to fail (see that mechanism's review-5). The state mechanisms of this record cause no held-out failure: BG 7/10 and BH 15/21 state pairs pass, every failure has the same or a higher difference count at merge, and 0 matches are lost in any cohort. Accept-ready once the canvas rounding obligation lands; the state verdicts below stand on their own.",
      cs_findings, cs_obligations,
      {"verdicts": {"outer image origin and size (f5442f4e5 -> f3ebc92f3)": "confirmed on held-out BG and BH; canvas rounding at the outer path is not Java's (int)(x + 1) (bounded obligation 1; within the frozen +-1 slack, no strict loss)",
                    "root-path note names (f5442f4e5 -> 3ebcb763e)": "confirmed (depths 1-3, duplicate texts, root plus group)",
                    "pass-three note anchors (d80bee422 -> ce1e52e30)": "confirmed (state only in Java; nested, root, concurrent, two notes before anchors)",
                    "routing guard addendum (f6b7f66f0)": "accepted: structural routing condition",
                    "above/below/floating outer notes": "residual as declared, confirmed"},
       "regressions": [],
       "failures": {"no5_three_notes_image / no6_two_left_same": "same-side second note is a Java link; pre-existing", "no5_note_top_image, no5_note_bottom_image, no6_top_depth2": "declared residual",
                    "no6_floating, rp6_named_floating_in_group": "declared residual (floating notes)", "no6_note_on_link_image": "routing of a note on a link; pre-existing",
                    "rp6_group_note_outer_anchor": "name now correct; pre-existing 3 px width"}})
write("tim-builtin-value-boundaries", 5, H["accounts"], H["clippy"],
      {"missing_glyph_and_width_decoding_draft": H["glyph"], "decode_site_escape_draft": H["decode"], "yaml_addendum": H["yaml_add"], "yaml_draft": H["yaml"], "clippy": H["clippy"], "m5a_request": H["m5a"]},
      "round-5 TIM/text obligations: missing-glyph advance from the macOS font chain, width paths decode carrier tokens, decode-site escaping (YAML/JSON/TIM JSON), YAML literal escapes, '?' for a unit in a comment",
      "Packet 8dda18b1b rejected. On held-out BH (Java exit 0) dc6_yaml_nested fails whole-output only because of a18a76980: Java keeps YAML escapes literal at parse time but Display.getWithNewlines then turns '\\\\' into '\\'; merge drew Java's 'a\\b', the tip draws 'a\\\\b'. The missing-glyph table holds on most held-out probes on the reference host but is falsified for unmapped nonspacing marks in complex text (3 BH cases, not regressions) and is host-dependent: generated on colossus, it differs from the reference host on 256 scalars (PingFang MobileAsset) and ignores user fonts that Java uses.",
      tim_findings, tim_obligations,
      {"verdicts": {"YAML literal escapes (a4e895aaf -> a18a76980)": "rejected: held-out BH dc6_yaml_nested lost to the packet (Display.getWithNewlines not ported)",
                    "missing-glyph advance (f5442f4e5 -> 4a16eaffd)": "not accepted: falsified for unmapped Mn marks in complex text (BH 3 cases) and host-dependent (PingFang asset, user fonts); constants confirmed as font metrics",
                    "width-token decoding (f5442f4e5 -> 4a16eaffd)": "confirmed",
                    "decode-site escaping JSON / TIM JSON (f5442f4e5 -> 673548fb9)": "confirmed",
                    "'?' for a unit in an SVG comment (comment-unit addendum)": "confirmed by 4 successful Java outputs; keep; the addendum's observability claim is false (UGroup.fix)"},
       "regressions": [{"case": "BH dc6_yaml_nested", "commit": H["yaml"], "java_exit": 0, "merge": "DIFF7 (inner 'a\\b' matched; the list item '\\u0041' was decoded)", "tip": "DIFF14, all from inner 'a\\\\b'", "java_mechanism": "TextBlockJson.getTextBlock 342 -> Display.getWithNewlines 284-305"}],
       "failures": {"dc6_yaml_nested": "caused by a18a76980", "w6_sans_180f_complex, w6_sans_1d17b, w6_sans_16fe4": "Mn mark zero advance not modelled; pre-existing wrong values",
                    "dc6_yaml_plain, key_newline, apostrophe, comment, hash_in_quotes, single, block": "Display.getWithNewlines not ported; pre-existing",
                    "w6_sans_211a, w6_sans_22ad, w6_sans_a8fb, w5_sans_a500, w6_sans_hebrew_missing": "mapped-advance residue (declared)",
                    "w6_sans_tag_flag, w6_times_e0080": "default-ignorable residue outside the declared scope; pre-existing",
                    "w5_mono_*, w6_mono_*": "family name 'monospace' and NBSP; widths match; pre-existing", "dc5_yaml_flow_escape": "declared flow residue",
                    "cm_unit_class_code, cm_unit_package_code, cm6_class_bare, cm6_class_low, cm6_object_code": "Rust code grammar drops the unit / entity; pre-existing"}})
