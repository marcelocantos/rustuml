import base64, gzip, hashlib, json, pathlib, sys
B = pathlib.Path(sys.argv[1])
AXES = {
 "raw_unit_test_input": ["T14.24", "exact input of renderer unit test parser_source_identity_matches_java_shadow_id_after_initial_blanks", "2 blanks after @startuml", "activity shadowing"],
 "raw_blank_before_start": ["T14.24", "2 blank lines before @startuml (Pipe NO_CONTENT discard)", "class shadowing"],
 "raw_trailing_blanks": ["T14.24", "2 blank lines before @enduml", "class shadowing"],
 "raw_crlf_one_blank": ["T14.24", "CRLF line terminators (CR occurs only before LF)", "1 blank after @startuml", "class shadowing"],
 "raw_multi_blank_first_block": ["T14.24", "multi-diagram stdin", "first block blank-only", "second block 1 leading blank, class shadowing"],
 "raw_include_leading_blanks": ["T14.24", "!include of a file starting with 2 blank lines", "class shadowing", "resource inc_leading_blank.iuml in PRIVATE_CWD"],
 "raw_whitespace_only_lines": ["T14.24", "space-only and tab-only lines after @startuml", "class shadowing"],
 "raw_sequence_two_blanks": ["T14.24", "2 blanks after @startuml", "sequence family shadowing", "note"],
 "delay_legacy_lead1": ["delay spacer", "legacy", "leading spacer run of 1"],
 "delay_legacy_plain0": ["delay spacer", "legacy", "no leading spacer", "medium label"],
 "delay_legacy_empty": ["delay spacer", "legacy", "empty label (bare ...)"],
 "delay_teoz_empty": ["delay spacer", "teoz", "empty label (bare ...)"],
 "delay_legacy_multiline": ["delay spacer", "legacy", "two-line label via \\n", "no leading spacer"],
 "delay_teoz_lead3_multiline": ["delay spacer", "teoz", "leading spacer run of 3", "two-line label"],
 "delay_teoz_lead3": ["delay spacer", "teoz", "leading spacer run of 3"],
 "delay_legacy_lead2_creole": ["delay spacer", "legacy", "leading spacer run of 2 before creole bold"],
 "cluster_root_note_on_inner": ["cluster autarky", "root-level note attached to inner leaf (note link crosses group)"],
 "cluster_inputpin_mixed_case": ["cluster autarky", "direct leaf <<InputPin>> mixed case (EntityPosition.fromStereotype equalsIgnoreCase)"],
 "cluster_entry_inside_autarkic": ["cluster autarky", "entryPoint leaf in nested group", "outer group with only inner links"],
 "history_external_suffix_form": ["history identity", "Vault[H] and Vault[H*] suffix endpoints from root (getHistorical(location, idShort))"],
 "cluster_concurrent_crossing": ["cluster autarky", "concurrent regions", "crossing link into a region"],
 "deploy_alias_direction_matrix": ["deployment link reversal", "explicit alias equal to projection", "left-head with left direction, right-head with left direction, both heads, undecorated dotted, left-head with down direction"],
 "deploy_nested_alias_reverse": ["deployment identity", "nested aliased node", "left-head and up-direction dotted reversal"],
 "stroke_stereo_style_css4": ["stroke scope", "<style> stereotype selector LineColor CSS4-only name"],
 "stroke_stereo_skinparam_css4": ["stroke scope", "skinparam state<<stereo>> BorderColor CSS4-only name"],
 "stroke_per_state_css4": ["stroke scope", "per-state ##CSS4-only line color"],
 "stroke_per_arrow_css4": ["stroke scope", "per-arrow -[#CSS4-only]-> color"],
 "stroke_arrow_skinparam_transparent": ["stroke scope", "skinparam ArrowColor transparent"],
 "stroke_zero_alpha_nonblack": ["stroke scope", "state style LineColor #FF000000 (zero alpha, non-black RGB)"],
 "theme_plain_sequence": ["unbundled theme", "!theme plain", "sequence"],
 "theme_plain_class_note": ["unbundled theme", "!theme plain", "class with note"],
 "theme_mono_class": ["unbundled theme", "!theme mono", "class"],
}
INV = ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"]
cases, files = [], []
for f in sorted((B / "sources").glob("*.puml")):
    data = f.read_bytes(); h = hashlib.sha256(data).hexdigest()
    cases.append({"name": f.stem, "source": str(f), "source_sha256": h, "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
                  "invocation": INV, "env": {}, "intended_valid": True, "axes": AXES[f.stem]})
    files.append({"path": f.stem + ".puml", "sha256": h, "base64": base64.b64encode(data).decode()})
assert len(cases) == len(AXES)
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
            "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": cases}
mbytes = json.dumps(manifest, indent=1, sort_keys=True).encode()
(B / "reference-manifest.json").write_bytes(mbytes)
res = []
for r in sorted((B / "resources").iterdir()):
    d = r.read_bytes(); res.append({"path": r.name, "sha256": hashlib.sha256(d).hexdigest(), "base64": base64.b64encode(d).decode()})
bundle = {"manifest": manifest, "manifest_sha256": hashlib.sha256(mbytes).hexdigest(), "files": files, "resources": res}
(B / "reference-input-bundle.json.gz").write_bytes(gzip.compress(json.dumps(bundle, sort_keys=True).encode(), mtime=0))
print(len(cases), "cases", len(res), "resources")
