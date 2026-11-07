"""Stage the round-3 maker held-out Java reference request (batch mk3).

Usage: python3 -I stage_mk3.py ROOT   (creates ROOT/mk3, which must not exist)
Every case is a renamed or held-out variant of an AT failure shape; none is in the
golden corpus or an earlier batch.
"""
import base64, gzip, hashlib, json, pathlib, sys

ROOT = pathlib.Path(sys.argv[1]); BATCH = ROOT / "mk3"; INPUTS = BATCH / "inputs"
INPUTS.mkdir(parents=True, exist_ok=False)
SP = "sprite-command-post-tim-ownership"; PA = "sequence-participant-silent-residue"
TEOZ = "!pragma teoz true\n"
cases = []
def add(name, body, valid, axes, mech):
    cases.append((name, "@startuml\n" + body + "@enduml\n", valid, axes, mech))

PIX = "sprite $tile [4x4/16] {\nF00F\n0FF0\n0FF0\nF00F\n}\n"
SVG = "sprite $mark <svg width=\"4\" height=\"4\">\n<rect width=\"2\" height=\"2\"/>\n</svg>\n"

# A. Bodies of multiline commands selected before (or instead of) the sprite group.
add("mk3_component_caption_body", "caption\nSprite atlas ready\nendcaption\n[Engine]\n", True, ["caption body", "file-form text", "component"], SP)
add("mk3_usecase_right_header_body", "right header\nsprite pool warmed\nendheader\nactor Hero\nHero -- (Play)\n", True, ["positioned header body", "lowercase file-form", "usecase"], SP)
add("mk3_deployment_center_footer_pixel_lookalike", "node Server\ncenter footer\nSPRITE $x [2x2/16] {\nend footer\n", True, ["positioned footer body", "pixel header lookalike", "deployment"], SP)
add("mk3_component_title_svg_lookalike", "title\nsprite $s <svg width=\"2\">\nendtitle\n[Engine]\n", True, ["title body", "svg header lookalike", "component"], SP)
add("mk3_deployment_legend_top_left_body", "node Server\nlegend top left\nSprite cache warm\nendlegend\n", True, ["legend with position", "file-form", "deployment"], SP)
add("mk3_usecase_bracket_body", "actor Hero\nusecase U1 [\nfirst step\nSprite deck shuffled\n]\nHero -- U1\n", True, ["description [ ] body", "usecase element", "usecase"], SP)
add("mk3_component_as_quote_body", "component C as \"first\nSprite text shown\nlast\"\n[C] --> [D]\n", True, ["description as \"...\" body (TYPE0)", "component"], SP)
add("mk3_deployment_database_bracket_upper", "database Store [\nrows\nSPRITE ROWS FLUSHED\n]\nnode App\nApp --> Store\n", True, ["description [ ] body", "uppercase", "deployment"], SP)
add("mk3_component_note_of_body", "[Engine]\nnote left of Engine\nSprite sheets cached\nend note\n", True, ["note of entity body", "component"], SP)
add("mk3_deployment_note_on_link_body", "node A\nnode B\nA --> B\nnote on link\nSprite link text\nend note\n", True, ["note on link body", "deployment"], SP)
add("mk3_component_floating_note_body", "[Engine]\nnote as N1\nsprite keeper online\nend note\n", True, ["floating note body", "component"], SP)
add("mk3_component_map_body", "map Config {\nSprite pool => full\nmode => fast\n}\n[Engine]\n", True, ["map body", "component"], SP)
add("mk3_deployment_archimate_body", "archimate #Business Plan [\nfirst\nSprite plan text\n]\n", True, ["archimate [ ] body", "deployment"], SP)
add("mk3_deployment_nested_package_header", "node Rack {\nnode Box\nheader\nSprite sheets are loaded\nendheader\n}\n", True, ["header body inside a package", "deeper nesting", "deployment"], SP)
add("mk3_component_note_embedded_end_lookalike", "[Engine]\nnote as N2\n{{\nend note\n}}\nSprite after embedded\nend note\n", True, ["note body with embedded diagram", "END lookalike inside {{ }}", "component"], SP)
add("mk3_state_caption_body", "state Idle\ncaption\nSprite atlas ready\nendcaption\n", True, ["caption body", "state"], SP)
add("mk3_state_left_footer_pixel_lookalike", "state Idle\nleft footer\nsprite $x [4x4/16] {\nendfooter\n", True, ["positioned footer body", "pixel header lookalike", "state"], SP)
add("mk3_state_note_on_link_body", "state Idle\nstate Busy\nIdle --> Busy\nnote on link\nSprite link text\nend note\n", True, ["note on link body", "state"], SP)
add("mk3_state_floating_note_body", "state Idle\nnote as N1\nSprite keeper online\nend note\n", True, ["floating note body", "state"], SP)
add("mk3_state_composite_header_body", "state Outer {\nheader\nSprite sheets are loaded\nendheader\nstate Inner\n}\n", True, ["header body inside a composite", "deeper nesting", "state"], SP)
add("mk3_state_json_then_sprite", "json J {\n\"a\": {\n\"b\": 1\n}\n}\n" + PIX + "state Idle : <$tile> ready\n", True, ["json body extends to valid JSON", "sprite after json", "state"], SP)
add("mk3_object_title_body", "object Player\ntitle\nSprite atlas ready\nend title\n", True, ["title body", "object"], SP)
add("mk3_object_caption_body", "caption\nSPRITE QUEUE FLUSHED\nendcaption\nobject Player\n", True, ["caption body", "uppercase", "object"], SP)
add("mk3_object_note_top_body", "object Player\nnote top of Player\nSprite sheets cached\nend note\n", True, ["note of entity body", "object"], SP)
add("mk3_object_map_body", "map Config {\nSprite pool => full\n}\nobject Player\n", True, ["map body", "object"], SP)
add("mk3_class_footer_svg_lookalike", "class Player\nfooter\nsprite $s <svg width=\"2\">\nendfooter\n", True, ["footer body", "svg header lookalike", "class"], SP)
add("mk3_class_body_separate_bracket", "class Player\n{\nSprite sheets loaded\n}\n", True, ["class body with { on its own line", "class"], SP)
add("mk3_class_tip_body", "class Player {\n  name : String\n}\nnote left of Player::name\nSprite tip text\nend note\n", True, ["member tip body", "class"], SP)
add("mk3_activity_legacy_note_left_upper", "(*) --> \"load\"\nnote left\nSPRITE ROWS\nend note\n\"load\" --> (*)\n", True, ["legacy activity note body", "uppercase"], SP)
add("mk3_activity_legacy_long_link_body", "(*) --> \"load\nSprite sheets are loaded\" as L\nL --> (*)\n", True, ["legacy multiline link label", "file-form body"], SP)
add("mk3_activity_legacy_header_body", "header\nSprite sheets are loaded\nendheader\n(*) --> \"load\"\n\"load\" --> (*)\n", True, ["legacy activity header body"], SP)
add("mk3_activity_legacy_caption_svg_lookalike", "(*) --> \"load\"\n\"load\" --> (*)\ncaption\nsprite $s <svg width=\"2\">\nendcaption\n", True, ["legacy activity caption body", "svg header lookalike"], SP)
add("mk3_seq_caption_body_legacy", "caption\nSprite sheets are loaded\nendcaption\nAlder -> Birch : go\n", True, ["caption body", "sequence legacy control"], SP)
add("mk3_seq_caption_body_teoz", TEOZ + "caption\nSprite sheets are loaded\nendcaption\nAlder -> Birch : go\n", True, ["caption body", "sequence teoz control"], SP)
add("mk3_component_sprite_used_after_bodies", "header\nbody\nendheader\n" + PIX + "[<$tile> Engine]\n", True, ["sprite after an owned body is still a command", "component"], SP)

# B. Apostrophes inside open sprite commands.
add("mk3_svg_end_apos_usecase", "sprite $mark <svg width=\"4\" height=\"4\">\n<text x=\"0\" y=\"3\"> 'v'</text></svg>\nactor Hero\n", True, ["svg END line with ' text", "usecase"], SP)
add("mk3_svg_mid_apos_state", "sprite $mark <svg width=\"4\" height=\"4\">\n<g> 'x' </g>\n</svg>\nstate Idle\n", True, ["svg body line with ' text", "state"], SP)
add("mk3_svg_end_trailing_deployment", "sprite $mark <svg width=\"4\" height=\"4\">\n<rect width=\"1\" height=\"1\"/>\n</svg> ' closed\nnode Server\n", False, ["text after </svg> on END line", "deployment"], SP)
add("mk3_pixel_end_trailing_class", "sprite $tile [2x2/16] {\nFF\nFF\n} ' done\nclass Player\n", False, ["text after } on pixel END line", "class"], SP)
add("mk3_pixel_end_trailing_seq_legacy", "sprite $tile [2x2/16] {\nFF\nFF\n} ' done\nAlder -> Birch : go\n", False, ["text after } on pixel END line", "sequence legacy"], SP)
add("mk3_svg_start_trailing_apos_component", "sprite $mark <svg width=\"4\" height=\"4\"> ' c\n<rect width=\"1\" height=\"1\"/>\n</svg>\n[Engine]\n", True, ["' text after <svg ...> on the start line", "component"], SP)
add("mk3_svg_inline_apos_object", "sprite $mark <svg width=\"4\" height=\"4\"><text> 'o'</text></svg>\nobject Player\n", True, ["inline svg with ' text", "object"], SP)

# C. Same-file !startsub sprite bodies.
add("mk3_startsub_pixel_component", "!startsub ICONS\n" + PIX + "!endsub\n[<$tile> Engine]\n", True, ["!startsub pixel sprite", "component"], SP)
add("mk3_startsub_svg_usecase", "!startsub ICONS\n" + SVG + "!endsub\nactor Hero\n", True, ["!startsub svg sprite", "usecase"], SP)
add("mk3_startsub_inside_false_if_deployment", "!if %false()\n!startsub ICONS\n" + PIX + "!endsub\n!endif\nnode \"<$tile> Server\" as S\n", True, ["!startsub inside !if false", "deployment"], SP)
add("mk3_startsub_pixel_seq_legacy", "!startsub ICONS\n" + PIX + "!endsub\nparticipant \"<$tile> Alder\" as A\nA -> A : go\n", True, ["!startsub pixel sprite", "sequence legacy"], SP)
add("mk3_startsub_pixel_state", "!startsub ICONS\n" + PIX + "!endsub\nstate Idle : <$tile> ready\n", True, ["!startsub pixel sprite", "state"], SP)
add("mk3_startsub_two_blocks_component", "!startsub A\n" + PIX + "!endsub\n!startsub B\n[Engine]\n!endsub\n[<$tile> Store]\n", True, ["two !startsub blocks in place", "component"], SP)

# D. Participant link tooltips (UrlBuilder#getUrl alternatives, STRICT).
add("mk3_tooltip_only_legacy", "participant Cedar [[{only tip}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["ONLY_TOOLTIP"], PA)
add("mk3_tooltip_and_label_legacy", "participant Cedar [[{tip} label]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["ONLY_TOOLTIP_AND_LABEL"], PA)
add("mk3_tooltip_quoted_label_legacy", "participant Cedar [[\"https://example.org/q\"{ q tip } label]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["QUOTED with tooltip and label", "untrimmed"], PA)
add("mk3_tooltip_with_label_legacy", "participant Cedar [[https://example.org/x{tip} label text]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["link, tooltip and label"], PA)
add("mk3_tooltip_space_before_brace_legacy", "participant Cedar [[https://example.org/x {spaced}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["LINK_TOOLTIP_NOLABEL with space before {"], PA)
add("mk3_tooltip_two_groups_legacy", "participant Cedar [[https://example.org/x{a}{b}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["LINK_TOOLTIP_NOLABEL greedy across two groups"], PA)
add("mk3_tooltip_label_brace_actor_legacy", "actor Moss [[https://example.org/m moss{y}]]\nparticipant Cedar\nMoss -> Cedar : go\n", True, ["label owns {y}", "actor"], PA)
add("mk3_tooltip_quoted_display_legacy", "participant \"Long name\" as L [[https://example.org/l{ l }]]\nparticipant Moss\nL -> Moss : go\n", True, ["quoted display", "spaced tooltip"], PA)
add("mk3_tooltip_empty_database_legacy", "database Store [[https://example.org/s{}]]\nparticipant Moss\nMoss -> Store : go\n", True, ["empty tooltip", "database"], PA)
add("mk3_tooltip_nested_teoz", TEOZ + "participant Cedar [[https://example.org/ring{a{b}c}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["nested braces", "teoz"], PA)

# E. Re-declarations (putParticipantInLast / addWithOrder).
add("mk3_redeclare_default_moves_legacy", "participant Alder\nparticipant Birch\nparticipant Alder\nAlder -> Birch : go\n", True, ["default order: re-declared participant moves last"], PA)
add("mk3_redeclare_negative_order_legacy", "participant Alder order -5\nparticipant Birch\nparticipant Alder\nAlder -> Birch : go\n", True, ["negative explicit order keeps first place"], PA)
add("mk3_redeclare_implicit_then_order_legacy", "Alder -> Birch : go\nparticipant Birch order 5\nBirch -> Alder : back\n", True, ["implicit participant re-declared with an order"], PA)
add("mk3_redeclare_higher_order_teoz", TEOZ + "participant Alder order 5\nparticipant Birch order 9\nparticipant Cedar order 7\nparticipant Birch\nAlder -> Birch : go\n", True, ["explicit orders, re-declared highest keeps last", "teoz"], PA)
add("mk3_redeclare_into_box_legacy", "participant Alder\nbox \"Zone\"\nparticipant Birch\nparticipant Alder\nend box\nAlder -> Birch : go\n", True, ["re-declaration inside a box moves into it"], PA)

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
