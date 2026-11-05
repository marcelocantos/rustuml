"""Stage checker batch ck3 (round-3 sprite/participant review) as an immutable Java reference request.

Usage: python3 -I stage_ck3.py BATCH_DIR
Writes BATCH_DIR/inputs/*.puml, BATCH_DIR/reference-manifest.json and BATCH_DIR/reference-input-bundle.json.gz.
No Java is run here. Validity is decided later from raw Java SVG + stderr, not from intended_valid.
"""
import base64, gzip, hashlib, json, sys
from pathlib import Path

BATCH = Path(sys.argv[1])
BATCH.mkdir(exist_ok=False)
INPUTS = BATCH / "inputs"
INPUTS.mkdir()
SP = "sprite-command-post-tim-ownership"
PA = "sequence-participant-silent-residue"
TEOZ = "!pragma teoz true\n"
PIX = "sprite $dot [2x2/16] {\nF0\n0F\n}\n"
cases, resources = [], []


def add(name, body, valid, axes, mech, wrap=True):
    src = f"@startuml\n{body}@enduml\n" if wrap else body
    cases.append((name, src, valid, axes, mech))


def resource(path, text):
    resources.append((path, text))


# A. Body ownership (command_spans walk): multiline owners and shapes not in AV/AT.
add("ck3_component_skinparam_block_sprite_line", "skinparam component {\nBackgroundColor #EEEEEE\nsprite $x foo\n}\n[Engine]\n", True, ["skinparam { } body", "sprite-file lookalike", "component"], SP)
add("ck3_deployment_style_block_sprite_line", "<style>\nnode {\nBackgroundColor #EEEEEE\n}\n</style>\nnode Server\nnote right of Server\nSprite sheets are loaded\nend note\n", True, ["<style> then note-of body", "deployment"], SP)
add("ck3_component_type1_stereo_color_url", "component C <<svc>> #pink [[https://example.org/c]] [\nfirst line\nSprite sheets are loaded\n]\n", True, ["TYPE1 with stereotype, colour and url", "component"], SP)
add("ck3_usecase_business_actor_type1", "actor/ Clerk [\nSprite sheets are loaded\ndesk\n]\n(Use)\nClerk --> (Use)\n", True, ["TYPE1 actor/", "usecase"], SP)
add("ck3_component_note_left_of_bracket_code", "[Comp]\nnote left of [Comp]\nSprite frame here\nend note\n", True, ["note of [code] body", "component"], SP)
add("ck3_usecase_note_top_of_paren", "(Login)\nnote top of (Login)\nSprite sheets are loaded\nend note\n", True, ["note of (usecase) body", "usecase"], SP)
add("ck3_component_note_of_color_url", "[Comp]\nnote right of Comp #lightblue [[https://example.org/n]]\nSprite sheets are loaded\nend note\n", True, ["note of entity with colour and url", "component"], SP)
add("ck3_component_note_as_stereo_color", "[Comp]\nnote as N1 <<memo>> #yellow\nsprite $z foo\nend note\nComp .. N1\n", True, ["note as code with stereotype and colour", "sprite-file lookalike", "component"], SP)
add("ck3_state_note_left_of_color", "state Idle\nnote left of Idle #pink\nSprite keeper online\nend note\n", True, ["state note of entity with colour", "state"], SP)
add("ck3_state_note_bracket_body", "state Idle\nnote right of Idle {\nSprite keeper online\n}\n", True, ["state note with { } body", "state"], SP)
add("ck3_class_generic_extends_body", "abstract class Box<T> extends Base {\nSprite sheets are loaded\n}\nclass Base\n", True, ["class generic + extends body", "class"], SP)
add("ck3_class_enum_upper_body", "ENUM Colors {\nSPRITE sheets loaded\nRED\n}\n", True, ["enum body", "uppercase", "class"], SP)
add("ck3_object_quoted_as_color_body", "object \"Order One\" as o1 #pink {\nSprite sheets = loaded\n}\n", True, ["object quoted as, colour", "object"], SP)
add("ck3_activity_legacy_note_on_link_body", "(*) --> \"Load\"\nnote on link\nSprite sheets are loaded\nend note\n\"Load\" --> (*)\n", True, ["legacy note on link body", "activity legacy"], SP)
add("ck3_activity_legacy_long_link_arrow_label", "(*) -->[go] \"multi\nSprite sheets are loaded\nline\"\n\"multi\\nSprite sheets are loaded\\nline\" --> (*)\n", True, ["legacy long link with arrow label", "activity legacy"], SP)
add("ck3_activity_legacy_partition_note", "partition Work {\n(*) --> \"Do\"\nnote left\nSprite stays here\nend note\n}\n", True, ["legacy note inside partition", "deeper nesting"], SP)
add("ck3_component_legend_nested_header_text", "[Engine]\nlegend\nheader\nSprite sheets are loaded\nendheader\nendlegend\n", True, ["legend body containing header lookalike", "nested"], SP)
add("ck3_state_header_embedded_end_lookalike", "state Idle\nheader\n{{\nendheader\n}}\nSprite keeper online\nendheader\n", True, ["header body with embedded diagram", "END lookalike inside {{ }}", "state"], SP)
add("ck3_deployment_unterminated_legend_at_enduml", "node Server\nlegend\nSprite sheets are loaded\n", False, ["unterminated legend body ends at @enduml", "deployment"], SP)
add("ck3_component_unterminated_type0_before_sprite", "component C as \"open\n" + PIX + "[Other]\n", False, ["unterminated TYPE0 body", "real sprite after", "component"], SP)
add("ck3_class_map_then_sprite_lookalike", "map Settings {\nSprite => loaded\n}\nclass Holder\n", True, ["map body", "class"], SP)
add("ck3_multi_diagram_header_then_sprite", "@startuml\nnode A\nheader\nSprite sheets are loaded\nendheader\n@enduml\n@startuml\n" + PIX + "node B\n@enduml\n", True, ["multi-diagram file", "deployment"], SP, wrap=False)
# Ungated families (walk not applied): header/legend/note bodies with sprite-file lookalikes.
add("ck3_timing_legend_body", "robust \"Web\" as W\n@0\nW is Idle\n@5\nW is Busy\nlegend\nSprite sheets are loaded\nendlegend\n", True, ["legend body", "timing"], SP)
add("ck3_mindmap_header_body", "@startmindmap\nheader\nSprite sheets are loaded\nendheader\n* root\n** leaf\n@endmindmap\n", True, ["header body", "mindmap"], SP, wrap=False)
add("ck3_wbs_caption_body", "@startwbs\ncaption\nSprite sheets are loaded\nendcaption\n* root\n** leaf\n@endwbs\n", True, ["caption body", "wbs"], SP, wrap=False)
add("ck3_gantt_title_body", "@startgantt\ntitle\nSprite sheets are loaded\nend title\n[Task] lasts 3 days\n@endgantt\n", True, ["title body", "gantt"], SP, wrap=False)
add("ck3_nwdiag_legend_body", "nwdiag {\nnetwork dmz {\nweb01\n}\n}\nlegend\nSprite sheets are loaded\nendlegend\n", True, ["legend body", "nwdiag"], SP)
add("ck3_activity3_note_body", "start\n:Load;\nnote right\nSprite sheets are loaded\nend note\nstop\n", True, ["note body", "activity3"], SP)
add("ck3_activity3_header_body", "header\nSprite sheets are loaded\nendheader\nstart\n:Load;\nstop\n", True, ["header body", "activity3"], SP)
add("ck3_seq_ref_body", "participant Alder\nparticipant Birch\nref over Alder, Birch\nSprite sheets are loaded\nend ref\n", True, ["ref body", "sequence legacy"], SP)

# B. Comment exemption: /' '/ spans, quoted apostrophes, leaks.
add("ck3_pixel_body_inner_comment_seq", "sprite $dot [2x2/16] {\nF0 /' row one '/\n0F\n}\nAlder -> Birch : <$dot>\n", True, ["pixel body line ending in /' '/", "sequence legacy"], SP)
add("ck3_svg_body_inner_comment_seq", "sprite $box <svg width=\"4\" height=\"4\">\n<rect width=\"4\" height=\"4\" fill=\"red\"/> /' fill '/\n</svg>\nAlder -> Birch : <$box>\n", True, ["svg body line ending in /' '/", "sequence legacy"], SP)
add("ck3_svg_start_inner_comment_teoz", TEOZ + "sprite $box <svg width=\"4\" height=\"4\"> /' start '/\n<rect width=\"4\" height=\"4\" fill=\"red\"/>\n</svg>\nAlder -> Birch : <$box>\n", True, ["svg start line ending in /' '/", "teoz"], SP)
add("ck3_block_comment_inside_pixel_body", "sprite $dot [2x2/16] {\n/' comment\nstill comment '/\nF0\n0F\n}\nAlder -> Birch : <$dot>\n", True, ["/' '/ block spanning pixel body lines", "sequence legacy"], SP)
add("ck3_svg_inline_quoted_apostrophe_seq", "sprite $txt <svg width=\"20\" height=\"10\"><text x=\"0\" y=\"8\">it ' s</text></svg>\nAlder -> Birch : <$txt> it's \"q ' q\"\n", True, ["' inside svg text and quoted label", "sequence legacy"], SP)
add("ck3_lookalike_leak_then_note_component", "[Engine]\nlegend\nsprite $x <svg width=\"4\">\nendlegend\nnote as N\nit's ' fine\nend note\n", True, ["svg start lookalike in legend body", "later ' text", "component"], SP)
add("ck3_lookalike_leak_then_message_seq", "Alder -> Birch : one\nnote over Alder\nsprite $p [2x2/16] {\nend note\nAlder -> Birch : two ' three\n", True, ["pixel start lookalike in note body", "later ' text", "sequence legacy"], SP)
add("ck3_svg_end_inner_comment_state", "sprite $box <svg width=\"4\" height=\"4\">\n<rect width=\"4\" height=\"4\" fill=\"red\"/>\n</svg> /' end '/\nstate Idle : <$box> ready\n", True, ["svg END line followed by /' '/", "state"], SP)

add("ck3_pixel_end_inner_comment_class", "sprite $dot [2x2/16] {\nF0\n0F\n} /' end '/\nclass Holder\n", True, ["pixel END line followed by /' '/", "class"], SP)
add("ck3_pixel_end_inner_comment_seq", "sprite $dot [2x2/16] {\nF0\n0F\n} /' end '/\nAlder -> Birch : <$dot>\n", True, ["pixel END line followed by /' '/", "sequence legacy"], SP)
add("ck3_svg_end_inner_comment_nospace_component", "sprite $box <svg width=\"4\" height=\"4\">\n<rect width=\"4\" height=\"4\" fill=\"red\"/>\n</svg>/'end'/\n[Engine]\n", True, ["svg END line glued to /' '/", "component"], SP)
add("ck3_svg_end_inner_comment_teoz", TEOZ + "sprite $box <svg width=\"4\" height=\"4\">\n<rect width=\"4\" height=\"4\" fill=\"red\"/>\n</svg> /' end '/\nAlder -> Birch : <$box>\n", True, ["svg END line followed by /' '/", "teoz"], SP)
add("ck3_svg_inline_inner_comment_object", "sprite $box <svg width=\"4\" height=\"4\"><rect width=\"4\" height=\"4\" fill=\"red\"/></svg> /' inline '/\nobject Store\n", True, ["single-line svg sprite followed by /' '/", "object"], SP)
add("ck3_pixel_start_inner_comment_deployment", "sprite $dot [2x2/16] { /' start '/\nF0\n0F\n}\nnode Server\n", True, ["pixel start line followed by /' '/", "deployment"], SP)
add("ck3_svg_end_inner_comment_usecase", "sprite $box <svg width=\"4\" height=\"4\">\n<rect width=\"4\" height=\"4\" fill=\"red\"/>\n</svg> /' end '/\nactor Hero\n", True, ["svg END line followed by /' '/", "usecase"], SP)

# C. !startsub in place (cross-family TIM).
resource("ck3lib.iuml", "!startsub AA\nAlder -> Birch : from AA\n!endsub\n!startsub BB\nBirch -> Cedar : from BB\n!endsub\nCedar -> Dove : outside subs\n")
resource("ck3pix.iuml", "!startsub ICON\n" + PIX + "!endsub\nAlder -> Alder : lib tail\n")
resource("puml-theme-ck3.puml", "skinparam ArrowColor #CC0000\n!startsub sequence\nskinparam ParticipantBackgroundColor #DDEEFF\n!endsub\n")
add("ck3_includesub_resource_one_sub_seq", "!includesub ck3lib.iuml!AA\nDove -> Alder : main\n", True, ["!includesub one sub of another file", "sequence legacy"], SP)
add("ck3_include_whole_with_subs_seq", "!include ck3lib.iuml\nDove -> Alder : main\n", True, ["!include of a file with subs: bodies in place", "sequence legacy"], SP)
add("ck3_include_then_includesub_seq", "!include ck3lib.iuml\n!includesub ck3lib.iuml!BB\n", True, ["!include then !includesub same sub", "sequence legacy"], SP)
add("ck3_includesub_pixel_sprite_seq", "!includesub ck3pix.iuml!ICON\nAlder -> Birch : <$dot>\n", True, ["!includesub pixel sprite", "sequence legacy"], SP)
add("ck3_nested_startsub_seq", "!startsub OUTER\nAlder -> Birch : outer\n!startsub INNER\nBirch -> Alder : inner\n!endsub\n!endsub\n", False, ["nested !startsub (Cannot nest sub)", "sequence legacy"], SP)
add("ck3_startsub_in_procedure_seq", "!procedure $twice()\n!startsub IN\nAlder -> Birch : in sub\n!endsub\nBirch -> Alder : after\n!endprocedure\n$twice()\n$twice()\n", True, ["!startsub inside a procedure invoked twice", "sequence legacy"], SP)
add("ck3_startsub_in_uninvoked_procedure_seq", "!procedure $never()\n!startsub IN\nAlder -> Birch : in sub\n!endsub\n!endprocedure\nAlder -> Birch : main\n", True, ["!startsub inside an uninvoked procedure", "sequence legacy"], SP)
add("ck3_startsub_in_while_seq", "!$i = 0\n!while $i < 2\n!startsub W\nAlder -> Birch : loop $i\n!endsub\n!$i = $i + 1\n!endwhile\n", True, ["!startsub inside !while", "sequence legacy"], SP)
add("ck3_startsub_endsub_in_false_if_seq", "!startsub S\nAlder -> Birch : one\n!if %false()\n!endsub\n!endif\nBirch -> Alder : two\n", False, ["!endsub inside !if false", "sequence legacy"], SP)
add("ck3_startsub_unterminated_seq", "Alder -> Birch : before\n!startsub OPEN\nBirch -> Alder : inside\n", False, ["unterminated !startsub", "sequence legacy"], SP)
add("ck3_theme_local_startsub_seq", "!theme ck3 from .\nAlder -> Birch : themed\n", True, ["local !theme with !startsub", "sequence legacy"], SP)
add("ck3_startsub_activity3", "start\n!startsub S\n:in sub;\n!endsub\n:after;\nstop\n", True, ["!startsub in place", "activity3"], SP)
add("ck3_startsub_mindmap", "@startmindmap\n* root\n!startsub M\n** child\n!endsub\n** sibling\n@endmindmap\n", True, ["!startsub in place", "mindmap"], SP, wrap=False)
add("ck3_startsub_gantt", "@startgantt\n!startsub G\n[Design] lasts 3 days\n!endsub\n[Build] lasts 2 days\n@endgantt\n", True, ["!startsub in place", "gantt"], SP, wrap=False)
add("ck3_startsub_timing", "robust \"Web\" as W\n!startsub T\n@0\nW is Idle\n!endsub\n@5\nW is Busy\n", True, ["!startsub in place", "timing"], SP)
add("ck3_startsub_salt", "@startsalt\n{\n!startsub S\n[Button]\n!endsub\n\"field\"\n}\n@endsalt\n", True, ["!startsub in place", "salt"], SP, wrap=False)
add("ck3_startsub_class", "!startsub C\nclass Alpha\n!endsub\nclass Beta\nAlpha --> Beta\n", True, ["!startsub in place", "class"], SP)
add("ck3_startsub_definelong_seq", "!definelong MSG\n!startsub D\nAlder -> Birch : in define\n!endsub\n!enddefinelong\nMSG\n", True, ["!startsub inside !definelong", "sequence legacy"], SP)

# D. UrlBuilder#getUrl (participants).
add("ck3_url_guillemet_strip_legacy", "participant Cedar [[«https://example.org/g»]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["Url quote strip with guillemets", "legacy"], PA)
add("ck3_url_spaces_inside_legacy", "participant Cedar [[ https://example.org/s ]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["spaces inside [[ ]]", "legacy"], PA)
add("ck3_url_quoted_spaces_teoz", TEOZ + "participant Cedar [[\"https://example.org/a b\"]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["QUOTED url with a space", "teoz"], PA)
add("ck3_url_topurl_legacy", "skinparam topurl https://base.example.org/\nparticipant Cedar [[page{tip}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["topurl relative link (declared residue)", "legacy"], PA)
add("ck3_url_uppercase_legacy", "PARTICIPANT Cedar [[HTTPS://EXAMPLE.ORG/P{Tip Text}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["uppercase keyword and link", "legacy"], PA)
add("ck3_url_label_only_actor_teoz", TEOZ + "actor Cedar [[https://example.org/l lbl]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["link with label only", "actor", "teoz"], PA)
add("ck3_url_unicode_tooltip_legacy", "participant Cedar [[https://example.org/é{tïp}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["non-ASCII url and tooltip", "legacy"], PA)
add("ck3_url_tooltip_label_teoz", TEOZ + "participant Cedar [[{tip} label]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["ONLY_TOOLTIP_AND_LABEL", "teoz"], PA)
add("ck3_url_entity_quoted_tooltip_legacy", "entity Cedar [[\"https://example.org/e\"{quoted tip}]]\nparticipant Moss\nCedar -> Moss : go\n", True, ["QUOTED with tooltip", "entity", "legacy"], PA)

# E. Re-declaration order (addWithOrder).
add("ck3_redeclare_tie_order_legacy", "participant Alder order 10\nparticipant Birch order 10\nparticipant Alder\nAlder -> Birch : go\n", True, ["equal explicit orders, re-declared first moves last", "legacy"], PA)
add("ck3_redeclare_explicit_zero_legacy", "participant Alder order 0\nparticipant Birch\nparticipant Alder\nAlder -> Birch : go\n", True, ["explicit order 0 ties default", "legacy"], PA)
add("ck3_redeclare_lower_order_teoz", TEOZ + "participant Alder order 3\nparticipant Birch order 1\nparticipant Cedar order 2\nparticipant Alder\nAlder -> Cedar : go\n", True, ["re-declared highest order keeps last", "teoz"], PA)
add("ck3_redeclare_out_of_box_legacy", "box \"Zone\"\nparticipant Alder\nparticipant Birch\nend box\nparticipant Alder\nAlder -> Birch : go\n", True, ["re-declaration after the box leaves it", "legacy"], PA)
add("ck3_redeclare_actor_kind_change_legacy", "participant Alder\nparticipant Birch\nactor Alder\nAlder -> Birch : go\n", True, ["re-declaration with another kind", "legacy"], PA)

for path, text in resources:
    assert not any(0xE000 <= ord(c) <= 0xE0FF or (ord(c) < 32 and c != "\n") for c in text), path
manifest_cases, files = [], []
for name, src, valid, axes, mech in cases:
    assert not any(0xE000 <= ord(c) <= 0xE0FF or (ord(c) < 32 and c != "\n") for c in src), name
    data = src.encode()
    p = INPUTS / f"{name}.puml"
    p.write_bytes(data)
    h = hashlib.sha256(data).hexdigest()
    manifest_cases.append({"name": name, "source": str(p), "source_sha256": h, "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
        "invocation": ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"],
        "env": {}, "intended_valid": valid, "axes": axes, "mechanism": mech})
    files.append({"path": f"{name}.puml", "sha256": h, "base64": base64.b64encode(data).decode()})
assert len({c["name"] for c in manifest_cases}) == len(manifest_cases)
res = []
for path, text in resources:
    data = text.encode()
    (INPUTS / path).write_bytes(data)
    res.append({"path": path, "sha256": hashlib.sha256(data).hexdigest(), "base64": base64.b64encode(data).decode()})
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
    "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": manifest_cases}
mbytes = json.dumps(manifest, indent=1, ensure_ascii=False).encode()
(BATCH / "reference-manifest.json").write_bytes(mbytes)
bundle = {"manifest": manifest, "manifest_sha256": hashlib.sha256(mbytes).hexdigest(), "files": files, "resources": res}
with gzip.GzipFile(BATCH / "reference-input-bundle.json.gz", "wb", mtime=0) as fh:
    fh.write(json.dumps(bundle, ensure_ascii=False).encode())
print(len(cases), sum(c[2] for c in cases), len(res))
