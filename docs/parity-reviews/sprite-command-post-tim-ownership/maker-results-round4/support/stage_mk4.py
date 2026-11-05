"""Stage maker batch mk4 (round-4 removeInnerComment port) as an immutable Java reference request.

Usage: python3 -I stage_mk4.py BATCH_DIR
Writes BATCH_DIR/inputs/*.puml, BATCH_DIR/reference-manifest.json and BATCH_DIR/reference-input-bundle.json.gz.
No Java is run here. Validity is decided later from raw Java SVG + stderr, not from intended_valid.
"""
import base64, gzip, hashlib, json, sys
from pathlib import Path

BATCH = Path(sys.argv[1])
BATCH.mkdir(exist_ok=False)
INPUTS = BATCH / "inputs"
INPUTS.mkdir()
SVG1 = 'sprite $box <svg width="4" height="4">'
RECT = '<rect width="4" height="4" fill="red"/>'
cases = []


def add(name, body, valid, axes):
    cases.append((name, f"@startuml\n{body}@enduml\n", valid, axes))


def add_raw(name, src, valid, axes):
    cases.append((name, src, valid, axes))


# A. Sprite lines (held out from AX): SVG body lines, /'''word''' spans, other families.
add("mk4_svg_body_trailing_comment_class", f"{SVG1}\n{RECT} /' body '/\n</svg>\nclass Ledger\n", True,
    ["svg body line", "trailing /' '/", "class"])
add("mk4_svg_body_leading_comment_usecase", f"{SVG1}\n/' lead '/{RECT}\n</svg>\n(Checkout)\n", True,
    ["svg body line", "leading /' '/ (PLAIN, tail kept)", "usecase"])
add("mk4_svg_body_two_comments_object",
    f"{SVG1}\n<rect width=\"2\" height=\"2\" fill=\"red\"/>/'a'/\n<rect x=\"2\" width=\"2\" height=\"2\" fill=\"blue\"/> /' b '/\n</svg>\nobject Vault\n",
    True, ["svg body lines", "two trailing spans, one without space", "object"])
add("mk4_svg_body_triple_word_component",
    f"{SVG1}\n<rect width=\"4\" /'''w-1'''/height=\"4\" fill=\"red\"/>\n</svg>\n[Gearbox]\n", True,
    ["svg body line", "mid-line /'''word'''/ removed", "component"])
add("mk4_svg_body_triple_spaced_state",
    f"{SVG1}\n{RECT} /''' kept '''/ <rect width=\"1\" height=\"1\" fill=\"blue\"/>\n</svg>\nstate Waiting\n", True,
    ["svg body line", "mid-line /''' spaced '''/ kept (\\w only)", "state"])
add("mk4_svg_end_triple_word_seq", f"{SVG1}\n{RECT}\n</svg>/'''end'''/\nAlder -> Birch : ok\n", True,
    ["svg END line", "trailing /'''word'''/ (lastIndexOf /')", "sequence"])
add("mk4_svg_inline_leading_comment_deployment",
    f"/' lead '/ {SVG1}{RECT}</svg>\nnode Rack\n", True,
    ["single-line svg sprite", "leading /' '/ before the sprite command", "deployment"])
add("mk4_svg_start_trailing_comment_activity3",
    f"{SVG1} /' s '/\n{RECT}\n</svg>\nstart\n:prepare;\nstop\n", True,
    ["svg start line", "trailing /' '/", "activity3"])
add("mk4_pixel_row_triple_word_class", "sprite $dot [2x2/16] {\nF/'''r1'''/0\n0F\n}\nclass Cask\n", True,
    ["pixel row", "mid-line /'''word'''/ removed", "class"])
add("mk4_pixel_row_triple_spaced_component", "sprite $dot [2x2/16] {\nF/''' r '''/0\n0F\n}\n[Valve]\n", True,
    ["pixel row", "mid-line /''' spaced '''/ kept (\\w only)", "component"])
add("mk4_pixel_row_triple_spaced_trailing_deployment", "sprite $dot [2x2/16] {\nF0 /''' r '''/ \n0F\n}\nnode Bay\n", True,
    ["pixel row", "/''' spaced '''/ then a space: Java trim ends with '/, trailing branch removes it", "deployment"])
add("mk4_pixel_row_trailing_nospace_usecase", "sprite $dot [2x2/16] {\nF0\n0F/'b'/\n}\n(Refund)\n", True,
    ["pixel row", "trailing /' '/ without space", "usecase"])
add("mk4_pixel_start_nospace_comment_state", "sprite $dot [2x2/16] {/'s'/\nF0\n0F\n}\nstate Parked\n", True,
    ["pixel start line", "trailing /' '/ without space", "state"])
add("mk4_pixel_end_leading_comment_object", "sprite $dot [2x2/16] {\nF0\n0F\n/' close '/ }\nobject Crate\n", True,
    ["pixel END line", "leading /' '/ before }", "object"])
add("mk4_pixel_end_triple_word_deployment", "sprite $dot [2x2/16] {\nF0\n0F\n}/'''x'''/\nnode Shelf\n", True,
    ["pixel END line", "trailing /'''word'''/", "deployment"])
add("mk4_pixel_end_two_spans_class", "sprite $dot [2x2/16] {\nF0\n0F\n} /' a '/ /' b '/\nclass Bin\n", False,
    ["pixel END line", "two trailing spans: removeInnerComment runs once, '} /' a '/' is not END", "class"])

add("mk4_pixel_row_triple_word_ref_component", "sprite $dot [2x2/16] {\nF/'''r1'''/0\n0F\n}\n[<$dot> Valve]\n", True,
    ["pixel row", "mid-line /'''word'''/ removed", "sprite referenced", "component"])
add("mk4_pixel_row_trailing_ref_class", "sprite $dot [2x2/16] {\nF0 /' r '/\n0F\n}\nclass \"<$dot> Keg\"\n", True,
    ["pixel row", "trailing /' '/", "sprite referenced", "class"])
add("mk4_pixel_leading_start_ref_state", "/' s '/ sprite $dot [2x2/16] {\nF0\n0F\n}\nstate \"<$dot> Dock\" as Dock\n", True,
    ["pixel start line", "leading /' '/ (was dropped whole)", "sprite referenced", "state"])

# B. Non-sprite lines across families (Rust strip_inline_comment cut at the apostrophe before).
add("mk4_seq_message_trailing_comment", "Alder -> Birch : hello /' note '/\n", True,
    ["sequence message", "trailing /' '/"])
add("mk4_seq_message_leading_comment", "/' lead '/ Alder -> Birch : hi\n", True,
    ["sequence message", "leading /' '/ (was dropped whole)"])
add("mk4_seq_quoted_comment", "Alder -> Birch : \"quoted /' q '/\n", True,
    ["sequence message", "span inside an open double quote (no quote handling)"])
add("mk4_seq_participant_quote_then_comment", "participant \"Al /' x\" as Al /' y '/\nAl -> Birch : go\n", True,
    ["sequence participant", "/' inside quotes, last /' is the comment"])
add("mk4_seq_note_body_comment", "Alder -> Birch : go\nnote right\nfirst /' n '/\nsecond\nend note\n", True,
    ["sequence note body", "trailing /' '/"])
add("mk4_seq_apostrophe_not_comment", "Alder -> Birch : don't stop\nBirch -> Alder : it's \"a ' b\"\n", True,
    ["sequence message", "apostrophes that are not /' or '/ (unchanged)"])
add("mk4_class_member_comment", "class Larch {\n+name : String /' field '/\n+size() : int\n}\n", True,
    ["class body member", "trailing /' '/"])
add("mk4_class_leading_comment_decl", "/' c '/ class Spruce\nSpruce --> Larch\n", True,
    ["class declaration", "leading /' '/ (was dropped whole)"])
add("mk4_object_triple_in_quotes", "object \"Store/'''x'''/Front\" as S\n", True,
    ["object declaration", "/'''word'''/ inside quotes removed"])
add("mk4_component_trailing_nospace", "[Engine]/'c'/\n[Engine] --> [Pump]\n", True,
    ["component", "trailing /' '/ without space"])
add("mk4_state_transition_comment", "[*] --> Idle\nIdle --> Busy : go /' t '/\n", True,
    ["state transition label", "trailing /' '/"])
add("mk4_usecase_leading_comment_usecase", "/' a '/ (Pay)\n:Clerk: --> (Pay)\n", True,
    ["usecase element", "leading /' '/ (was dropped whole)"])
add("mk4_activity3_action_comment", "start\n:step one; /' c '/\n:step two;\nstop\n", True,
    ["activity3 action", "trailing /' '/"])
add_raw("mk4_mindmap_node_comment", "@startmindmap\n* root /' r '/\n** child\n@endmindmap\n", True,
        ["mindmap node", "trailing /' '/"])
add_raw("mk4_gantt_task_comment", "@startgantt\n[Build] lasts 3 days /' g '/\n[Test] lasts 2 days\n@endgantt\n", True,
        ["gantt task", "trailing /' '/"])
add_raw("mk4_json_member_comment", "@startjson\n{\n\"a\": 1, /' c '/\n\"b\": [1, 2]\n}\n@endjson\n", True,
        ["json body line", "trailing /' '/ (Rust left invalid JSON before)"])
add_raw("mk4_salt_widget_comment", "@startsalt\n{\n[Ok] /' b '/\n\"field\"\n}\n@endsalt\n", True,
        ["salt widget line", "trailing /' '/"])
add("mk4_class_skinparam_leading_comment", "/' c '/ skinparam monochrome true\nclass Pine\n", True,
    ["skinparam line", "leading /' '/ (directive kept)", "class"])

# C. TIM interaction: directive lines and bodies see removeInnerComment's text.
add("mk4_tim_define_comment", "!define WHO Birch /' w '/\nAlder -> WHO : hi\n", True,
    ["!define value", "trailing /' '/", "sequence"])
add("mk4_tim_procedure_body_comment",
    "!procedure $call($x)\nAlder -> $x : ping /' b '/\n!endprocedure\n$call(\"Birch\")\n", True,
    ["!procedure body", "trailing /' '/", "sequence"])
add("mk4_tim_if_comment", "!$on = 1\n!if $on == 1 /' c '/\nclass Fir\n!endif\n", True,
    ["!if condition", "trailing /' '/", "class"])
add("mk4_tim_sprite_in_procedure",
    "!procedure $mk()\nsprite $dot [2x2/16] {\nF0\n0F\n} /' e '/\n!endprocedure\n$mk()\nclass Oak\n", True,
    ["sprite inside !procedure body", "pixel END trailing /' '/", "class"])
add("mk4_tim_startsub_comment", "!startsub PART /' s '/\nclass Elm\n!endsub\nclass Ash\n", True,
    ["!startsub line", "trailing /' '/", "class"])

# D. Residue and mechanism probes.
add("mk4_leading_comment_ends_with_open", "/' a '/ Alder -> Birch : x /'\nBirch -> Alder : y\n", True,
    ["leading /' '/ on a line ending with /'", "Rust long-comment start residue (unchanged)", "sequence"])
add("mk4_svg_text_fill_apostrophe_seq",
    "sprite $txt <svg width=\"20\" height=\"10\"><text x=\"0\" y=\"8\" fill=\"black\">it ' s</text></svg>\nAlder -> Birch : <$txt> it's \"q ' q\"\n",
    True, ["svg <text> with fill", "apostrophe inside sprite text", "control for ck3_svg_inline_quoted_apostrophe_seq"])
add("mk4_svg_text_nofill_plain_seq",
    "sprite $txt <svg width=\"20\" height=\"10\"><text x=\"0\" y=\"8\">ab</text></svg>\nAlder -> Birch : <$txt> ok\n",
    False, ["svg <text> without fill", "no apostrophe", "SvgNanoParser#drawText null fill"])
add("mk4_svg_text_nofill_usecase",
    "sprite $txt <svg width=\"20\" height=\"10\"><text x=\"0\" y=\"8\">cd</text></svg>\n(<$txt> go)\n",
    False, ["svg <text> without fill", "usecase", "SvgNanoParser#drawText null fill"])
add("mk4_ctl_svg_end_state", f"{SVG1}\n{RECT}\n</svg>\nstate Idle : <$box> ready\n", True,
    ["control: ck3_svg_end_inner_comment_state without the comment", "state"])
add("mk4_ctl_pixel_end_seq", "sprite $dot [2x2/16] {\nF0\n0F\n}\nAlder -> Birch : <$dot>\n", True,
    ["control: ck3_pixel_end_inner_comment_seq without the comment", "sequence"])

manifest_cases, files = [], []
for name, src, valid, axes in cases:
    assert all(c == "\n" or ord(c) >= 0x20 for c in src), name
    assert not any(0xE000 <= ord(c) <= 0xE0FF for c in src), name
    data = src.encode()
    p = INPUTS / f"{name}.puml"
    p.write_bytes(data)
    h = hashlib.sha256(data).hexdigest()
    manifest_cases.append({"name": name, "source": str(p), "source_sha256": h, "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
                           "invocation": ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone",
                                          "GMT+11:00", "--", "-pipe", "-tsvg"],
                           "env": {}, "intended_valid": valid, "axes": axes})
    files.append({"path": f"{name}.puml", "sha256": h, "base64": base64.b64encode(data).decode()})
assert len({c["name"] for c in manifest_cases}) == len(manifest_cases)
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
            "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": manifest_cases}
mbytes = json.dumps(manifest, indent=1, ensure_ascii=False).encode()
(BATCH / "reference-manifest.json").write_bytes(mbytes)
bundle = {"manifest": manifest, "manifest_sha256": hashlib.sha256(mbytes).hexdigest(), "files": files, "resources": []}
with gzip.GzipFile(BATCH / "reference-input-bundle.json.gz", "wb", mtime=0) as fh:
    fh.write(json.dumps(bundle, ensure_ascii=False).encode())
for f in ("reference-manifest.json", "reference-input-bundle.json.gz"):
    print(hashlib.sha256((BATCH / f).read_bytes()).hexdigest(), f)
print(len(manifest_cases), sum(c["intended_valid"] for c in manifest_cases), "intended valid")
