"""Stage an immutable Java reference request (checker batch1) for the sprite ownership / participant review."""
import base64, gzip, hashlib, json, pathlib, sys

ROOT = pathlib.Path(sys.argv[1]); BATCH = ROOT / "batch1"; INPUTS = BATCH / "inputs"
INPUTS.mkdir(parents=True, exist_ok=False)
SP = "sprite-command-post-tim-ownership"; PA = "sequence-participant-silent-residue"
SEQ = {"legacy": "", "teoz": "!pragma teoz true\n"}
cases = []
def add(name, body, valid, axes, mech):
    cases.append((name, "@startuml\n" + body + "@enduml\n", valid, axes, mech))
def addraw(name, src, valid, axes, mech):
    cases.append((name, src, valid, axes, mech))

FILE = "Sprite sheets are loaded"   # CommandSpriteFile shape at a command start
# A. Multiline command bodies Java owns through END (never offered to getCandidate).
add("ck_component_bracket_body", f"component Engine [\nfirst line\n{FILE}\n]\n", True, ["description bracket body", "file-form text", "component"], SP)
add("ck_component_bracket_lower", "component Engine [\nheader row\nsprite pool holds frames\n]\n[Engine] --> [Store]\n", True, ["description bracket body", "lowercase file-form", "component"], SP)
add("ck_deployment_node_bracket_body", f"node Server [\nfirst line\n{FILE}\n]\n", True, ["description bracket body", "file-form text", "deployment"], SP)
add("ck_deployment_bracket_upper", "node Server [\nstatus\nSPRITE QUEUE FLUSHED NOW\n]\nnode Client\nClient --> Server\n", True, ["description bracket body", "uppercase file-form", "deployment"], SP)
add("ck_usecase_rectangle_bracket_body", f"actor Hero\nrectangle Stage [\nfirst line\n{FILE}\n]\nHero -- Stage\n", True, ["description bracket body", "file-form text", "usecase"], SP)
add("ck_component_header_body", f"header\n{FILE}\nendheader\n[Engine]\n", True, ["multiline header body", "component"], SP)
add("ck_component_footer_body", f"footer\n{FILE}\nendfooter\n[Engine]\n", True, ["multiline footer body", "component"], SP)
add("ck_deployment_header_body", f"header\n{FILE}\nendheader\nnode Server\n", True, ["multiline header body", "deployment"], SP)
add("ck_deployment_footer_upper", "node Server\nfooter\nSPRITE QUEUE FLUSHED NOW\nendfooter\n", True, ["multiline footer body", "uppercase", "deployment"], SP)
add("ck_usecase_header_body", f"header\n{FILE}\nendheader\nactor Hero\nHero -- (Play)\n", True, ["multiline header body", "usecase"], SP)
add("ck_state_header_body", f"header\n{FILE}\nendheader\nstate Idle\n", True, ["multiline header body", "state"], SP)
add("ck_state_legend_body", f"state Idle\nlegend\n{FILE}\nendlegend\n", True, ["legend body", "state"], SP)
add("ck_state_title_body", f"title\n{FILE}\nend title\nstate Idle\n", True, ["multiline title body", "state"], SP)
add("ck_object_header_body", f"header\n{FILE}\nendheader\nobject Player\n", True, ["multiline header body", "object"], SP)
add("ck_object_legend_body", f"object Player\nlegend\n{FILE}\nendlegend\n", True, ["legend body", "object"], SP)
add("ck_activity_legacy_note_body", f"(*) --> \"load\"\nnote right\n{FILE}\nend note\n\"load\" --> (*)\n", True, ["legacy activity note body", "file-form"], SP)
add("ck_activity_legacy_note_brace", "(*) --> \"load\"\nnote right\nSprite frame {\nend note\n\"load\" --> (*)\n", True, ["legacy activity note body", "pixel header lookalike"], SP)
add("ck_activity_legacy_legend_body", f"(*) --> \"load\"\n\"load\" --> (*)\nlegend\n{FILE}\nendlegend\n", True, ["legacy activity legend body"], SP)
add("ck_class_header_body", f"header\n{FILE}\nendheader\nclass Player\n", True, ["multiline header body", "class"], SP)
add("ck_class_legend_body", f"class Player\nlegend\n{FILE}\nendlegend\n", True, ["legend body", "class"], SP)
add("ck_class_floating_note_body", f"class Player\nnote as N1\n{FILE}\nend note\n", True, ["floating note body", "class"], SP)
add("ck_seq_header_body_legacy", f"header\n{FILE}\nendheader\nAlder -> Birch : go\n", True, ["multiline header body", "sequence legacy"], SP)
# B. Rust-only inline-comment stripping now reaches multiline SVG body lines.
APOS_END = "sprite $tag <svg width=\"4\" height=\"4\">\n<text> 'hi'</text></svg>\n"
for mode, pre in SEQ.items():
    add(f"ck_svg_body_apos_end_{mode}", pre + "participant Alder\nparticipant Birch\n" + APOS_END + "Alder -> Birch : go\n", True, ["multiline svg body", "space-apostrophe on END line", mode], SP)
    add(f"ck_svg_body_trailing_after_end_{mode}", pre + "participant Alder\nparticipant Birch\nsprite $tag <svg width=\"4\" height=\"4\">\n<rect width=\"1\" height=\"1\"/>\n</svg> ' done\nAlder -> Birch : go\n", False, ["multiline svg body", "text after </svg> on END line", mode], SP)
add("ck_svg_body_apos_end_class", APOS_END + "class Player\n", True, ["multiline svg body", "space-apostrophe on END line", "class"], SP)
add("ck_svg_body_apos_end_component", APOS_END + "[Engine]\n", True, ["multiline svg body", "space-apostrophe on END line", "component"], SP)
add("ck_svg_body_apos_mid_legacy", "participant Alder\nparticipant Birch\nsprite $tag <svg width=\"4\" height=\"4\">\n<text> 'hi'</text>\n</svg>\nAlder -> Birch : go\n", True, ["multiline svg body", "space-apostrophe mid body", "control"], SP)
add("ck_svg_body_trailing_after_end_class", "sprite $tag <svg width=\"4\" height=\"4\">\n<rect width=\"1\" height=\"1\"/>\n</svg> ' done\nclass Player\n", False, ["multiline svg body", "text after </svg> on END line", "class"], SP)
# C. !startsub bodies in the same file are active TIM output.
GEM = "sprite $gem [4x4/16] {\nF00F\n0FF0\n0FF0\nF00F\n}\n"
add("ck_startsub_sprite_deployment", "!startsub GEMS\n" + GEM + "!endsub\nnode \"<$gem> Server\" as S\n", True, ["!startsub body", "pixel sprite drawn", "deployment"], SP)
add("ck_nosub_sprite_deployment", GEM + "node \"<$gem> Server\" as S\n", True, ["control without startsub", "pixel sprite drawn", "deployment"], SP)
add("ck_startsub_sprite_class", "!startsub GEMS\n" + GEM + "!endsub\nclass Player {\n  <$gem> icon\n}\n", True, ["!startsub body", "pixel sprite", "class"], SP)
# D. Families without CommonCommands (Java passes sprite-looking text through).
addraw("ck_yaml_block_sprite_text", "@startyaml\ndesc: |\n  sprite sheet ready\nname: atlas\n@endyaml\n", True, ["yaml block scalar", "single-line sprite shape"], SP)
addraw("ck_ditaa_sprite_text", "@startditaa\n+--------+\n| hello  |\n+--------+\nsprite sheet ready\n@endditaa\n", True, ["ditaa text", "single-line sprite shape"], SP)
# E. Participant link tooltip (Url#getTooltip via UrlBuilder alternatives).
for mode, pre in SEQ.items():
    add(f"ck_tooltip_label_brace_{mode}", pre + "participant Cedar [[https://example.org/ring ring{x}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["url label containing braces", "no tooltip", mode], PA)
add("ck_tooltip_spaced_legacy", "participant Cedar [[https://example.org/ring{ rings }]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["tooltip with surrounding spaces"], PA)
add("ck_tooltip_empty_legacy", "participant Cedar [[https://example.org/ring{}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["empty tooltip"], PA)
add("ck_tooltip_nested_brace_legacy", "participant Cedar [[https://example.org/ring{a{b}c}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["nested braces in tooltip"], PA)
add("ck_create_tooltip_legacy", "participant Alder\nAlder -> Birch : hi\ncreate participant Cedar [[https://example.org/c{cedar tip}]]\nAlder -> Cedar : make\n", True, ["created participant", "tooltip"], PA)
# F. putParticipantInLast warning on re-declarations Java does not move.
for mode, pre in SEQ.items():
    add(f"ck_redeclare_order_stable_{mode}", pre + "participant Alder order 10\nparticipant Birch order 20\nparticipant Alder order 10\nAlder -> Birch : go\n", True, ["re-declaration", "explicit order keeps position"], PA)
add("ck_redeclare_order_lower_legacy", "participant Alder order 30\nparticipant Birch order 20\nparticipant Birch\nAlder -> Birch : go\n", True, ["re-declaration", "lower explicit order keeps position"], PA)
add("ck_redeclare_last_legacy", "participant Alder\nparticipant Birch\nparticipant Birch\nAlder -> Birch : go\n", True, ["re-declaration of last participant", "control"], PA)

manifest_cases, files = [], []
for name, src, valid, axes, mech in cases:
    assert not any(0xE000 <= ord(c) <= 0xE0FF or (ord(c) < 32 and c != "\n") for c in src), name
    data = src.encode(); p = INPUTS / f"{name}.puml"; p.write_bytes(data); h = hashlib.sha256(data).hexdigest()
    manifest_cases.append({"name": name, "source": str(p), "source_sha256": h, "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
        "invocation": ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"],
        "env": {}, "intended_valid": valid, "axes": axes, "mechanism": mech})
    files.append({"path": f"{name}.puml", "sha256": h, "base64": base64.b64encode(data).decode()})
assert len({c["name"] for c in manifest_cases}) == len(manifest_cases)
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
    "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": manifest_cases}
mbytes = json.dumps(manifest, indent=1, ensure_ascii=False).encode()
(BATCH / "reference-manifest.json").write_bytes(mbytes)
bundle = {"manifest": manifest, "manifest_sha256": hashlib.sha256(mbytes).hexdigest(), "files": files, "resources": []}
with gzip.GzipFile(BATCH / "reference-input-bundle.json.gz", "wb", mtime=0) as fh:
    fh.write(json.dumps(bundle, ensure_ascii=False).encode())
print(len(cases), sum(c[2] for c in cases))
