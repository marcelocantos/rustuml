"""Write the checker's round-5 probe batch c5a (held out from the maker). Usage: python3 -I c5a_sources.py <dir>"""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); cases = []
def add(name, text, axes, valid=True):
    (D / f"{name}.puml").write_text(text, encoding="utf-8"); cases.append({"name": name, "axes": axes, "intended_valid": valid})
def cls(label, pre=""):
    return f"@startuml\n{pre}class \"{label}\" as V\n@enduml\n"
U = chr
# --- width: model says Missing (simple path, LastResort) for assigned/unassigned BMP scalars in new blocks
for cp, why in [(0x0870, "Arabic Ext-B, simple path, LastResort"), (0x1C80, "Cyrillic Ext-C"), (0x2E5E, "Supplemental Punctuation, Unicode 14"),
                (0x1AB0, "combining ext, not a complex char"), (0x0D80, "Sinhala unassigned"), (0xA69E, "Cyrillic Ext-B, Java TNR zero"),
                (0x2065, "BMP default-ignorable unassigned, simple text"), (0xFFF0, "specials unassigned, simple text")]:
    add(f"w6_sans_{cp:04x}", cls("Vault " + U(cp)), ["missing-glyph", "simple-path", why])
# the same default-ignorables inside complex text (HarfBuzz hides them)
for cp in (0x2065, 0xFFF0, 0x180F):
    add(f"w6_sans_{cp:04x}_complex", cls("Vault " + U(cp) + U(0x10FFFD)), ["missing-glyph", "complex-path", "default-ignorable"])
# CoreText substitution in simple text (model: Mapped, advance unported): substitution itself is the question
for cp, why in [(0x211A, "Helvetica substitutes; Java TNR table says missing"), (0x22AD, "Apple Symbols substitutes; TNR says missing"),
                (0xA8FB, "dot-prefixed .SFDevanagari substitutes; TNR says missing")]:
    add(f"w6_sans_{cp:04x}", cls("Vault " + U(cp)), ["missing-glyph", "simple-path", "substitution", why])
# supplementary, complex path: cascade coverage
for cp, why in [(0x1E900, "Adlam; CoreText substitutes NotoSansAdlam, cascade?"), (0x1F600, "emoji, Apple Color Emoji in cascade"),
                (0x30000, "CJK Ext-G"), (0x31350, "CJK Ext-H"), (0x1FAE8, "emoji 15"), (0xE0080, "tags block unassigned, hidden"),
                (0xE01F0, "VS supplement unassigned, hidden"), (0x1D17B, "after HarfBuzz hidden run U+1D173..1D17A"),
                (0x16FE4, "Khitan filler"), (0x13430, "Egyptian format control")]:
    add(f"w6_sans_{cp:05x}", cls("Vault " + U(cp)), ["missing-glyph", "complex-path", why])
# sequences: ZWJ, flags, combining, RTL
add("w6_sans_zwj_family", cls("Vault \U0001F468\u200D\U0001F469\u200D\U0001F467"), ["missing-glyph", "zwj", "complex"])
add("w6_sans_tag_flag", cls("Vault \U0001F3F4\U000E0067\U000E0062\U000E0073\U000E0063\U000E0074\U000E007F"), ["missing-glyph", "tag-sequence", "hidden tags"])
add("w6_sans_combining", cls("Vault a\u0301\u0378"), ["missing-glyph", "combining makes complex", "unassigned in complex text"])
add("w6_sans_hebrew_missing", cls("Vault \u05D0\u05EB"), ["missing-glyph", "RTL", "U+05EB unassigned inside complex range"])
add("w6_sans_arabic_missing", cls("Vault \u0628\u061D\u0870"), ["missing-glyph", "RTL", "complex text, U+0870 via cascade"])
add("w6_sans_thai_missing", cls("Vault \u0E01\u0E3B"), ["missing-glyph", "complex", "Thai unassigned U+0E3B"])
# sizes and bold
for size in (9, 23):
    add(f"w6_sans_size{size}_0378", cls("Vault \u0378", f"skinparam classFontSize {size}\n"), ["missing-glyph", f"size {size}"])
    add(f"w6_sans_size{size}_10fffd", cls("Vault \U0010FFFD", f"skinparam classFontSize {size}\n"), ["missing-glyph", f"size {size}", "complex"])
add("w6_sans_bold_0870", cls("<b>Vault \u0870</b>"), ["missing-glyph", "bold creole", "simple"])
# other faces
for fam, tag in [("Arial", "arial"), ("Verdana", "verdana"), ("Helvetica", "helvetica"), ("Courier New", "couriernew"),
                 ("Monospaced", "mono"), ("Times New Roman", "times")]:
    pre = f"skinparam defaultFontName {fam}\n"
    add(f"w6_{tag}_0870", cls("Vault \u0870", pre), ["missing-glyph", fam, "simple"])
    add(f"w6_{tag}_e0080", cls("Vault \U000E0080", pre), ["missing-glyph", fam, "hidden"])
    add(f"w6_{tag}_1e900", cls("Vault \U0001E900", pre), ["missing-glyph", fam, "complex cascade"])
add("w6_arial_size11_10fffd", cls("Vault \U0010FFFD", "skinparam defaultFontName Arial\nskinparam classFontSize 11\n"), ["missing-glyph", "Arial size 11"])
add("w6_verdana_bold_10fffd", cls("<b>Vault \U0010FFFD</b>", "skinparam defaultFontName Verdana\n"), ["missing-glyph", "Verdana bold"])
# other renderers measuring text: sequence participant, activity, usecase, note, mindmap
add("w6_seq_0870", "@startuml\nparticipant \"Vault \u0870\" as V\nV -> V : ok \U0010FFFD\n@enduml\n", ["missing-glyph", "sequence"])
add("w6_activity_10fffd", "@startuml\n:Vault \U0010FFFD;\n:Vault \u0870;\n@enduml\n", ["missing-glyph", "activity"])
add("w6_state_e0080", "@startuml\nstate \"Vault \U000E0080 \u0870\" as V\n@enduml\n", ["missing-glyph", "state"])
add("w6_mindmap_0378", "@startmindmap\n* Vault \u0378\n** Leaf \U0010FFFD\n@endmindmap\n", ["missing-glyph", "mindmap"])
# --- decode sites: YAML literal escapes
Y = lambda body: f"@startyaml\n{body}@endyaml\n"
add("dc6_yaml_escaped_quote", Y('msg: "say \\"hi\\" now"\n'), ["yaml-unquote", "escaped quote inside double quotes"])
add("dc6_yaml_trailing_backslash", Y('path: "C:\\dir\\"\n'), ["yaml-unquote", "value ends with backslash-quote"])
add("dc6_yaml_key_newline", Y('"a\\nb": v\n'), ["yaml-unquote", "quoted key with \\n"])
add("dc6_yaml_apostrophe", Y('k: "it\'s \\t ok"\n'), ["yaml-unquote", "single quote inside double quotes"])
add("dc6_yaml_comment", Y('k: "x\\ty" # trailing\n'), ["yaml-unquote", "comment after quoted value"])
add("dc6_yaml_hash_in_quotes", Y('k: "a #\\n b"\n'), ["yaml-unquote", "hash inside quotes"])
add("dc6_yaml_nested", Y('outer:\n  inner: "a\\\\b"\n  list:\n    - "c\\u0041"\n'), ["yaml-unquote", "nested map and list"])
add("dc6_yaml_plain", Y('k: a\\nb\n'), ["yaml-unquote", "plain scalar control"])
add("dc6_yaml_single", Y("k: 'a\\nb'\n"), ["yaml-unquote", "single-quoted control"])
add("dc6_yaml_colon_key", Y('"a:b\\c": v\n'), ["yaml-unquote", "colon inside quoted key"])
add("dc6_yaml_block", Y('k: |\n  "a\\nb"\n  c\n'), ["yaml-unquote", "block scalar line"])
add("dc6_yaml_listonly", Y('- "x\\U0010FFFD y"\n- plain\n'), ["yaml-unquote", "list item with \\U escape"])
add("dc6_json_control", "@startjson\n{\"k\": \"a\\u0041\\\\b\"}\n@endjson\n", ["json decodes", "control"])
# --- comment unit in other families (Rust keeps the unit in the code?)
pre = '!$u = %substr("\U0001F680", 0, 1)\n'
add("cm6_state_code", f"@startuml\n{pre}state \"Shown\" as S$u\n@enduml\n", ["comment-unit", "state"])
add("cm6_object_code", f"@startuml\n{pre}object \"Shown\" as O$u\n@enduml\n", ["comment-unit", "object"])
add("cm6_usecase_code", f"@startuml\n{pre}usecase \"Shown\" as U$u\n@enduml\n", ["comment-unit", "usecase"])
add("cm6_component_code", f"@startuml\n{pre}component \"Shown\" as C$u\n@enduml\n", ["comment-unit", "component"])
add("cm6_class_bare", f"@startuml\n{pre}class Q$u\n@enduml\n", ["comment-unit", "class bare code, display has the unit"], valid=False)
add("cm6_class_low", '@startuml\n!$u = %substr("\U0001F680", 1, 1)\nclass "Shown" as Q$u\n@enduml\n', ["comment-unit", "low surrogate"])
# --- outer notes / origin (state image path)
add("no6_left_depth2", "@startuml\nstate Yard {\n  state Shed {\n    Rake --> Hoe\n  }\n}\nnote left of Yard : tools\n@enduml\n", ["outer-origin", "note left", "depth 2 image"])
add("no6_right_long", "@startuml\nstate Pier {\n  Boat --> Dock\n}\nnote right of Pier : a much longer note text here\n@enduml\n", ["outer-origin", "note right", "wide note"])
add("no6_left_and_start", "@startuml\n[*] --> Bay\nstate Bay {\n  Gull --> Tern\n}\nnote left of Bay : birds\nBay --> [*]\n@enduml\n", ["outer-origin", "note left", "start/end on outer"])
add("no6_left_right_both", "@startuml\nstate Loft {\n  Hay --> Oat\n}\nnote left of Loft : dry\nnote right of Loft : warm\n@enduml\n", ["outer-origin", "two notes, opposite sides"])
add("no6_two_left_same", "@startuml\nstate Vat {\n  Malt --> Wort\n}\nnote left of Vat : one\nnote left of Vat : two\n@enduml\n", ["outer-origin", "two notes same side (BG three-notes mechanism)"])
add("no6_note_on_link_image", "@startuml\nstate Mill {\n  Grain --> Flour\n}\nSack --> Mill\nnote on link : carry\n@enduml\n", ["outer-origin", "note on link"])
add("no6_floating", "@startuml\nstate Kiln {\n  Clay --> Pot\n}\nnote \"loose\" as N1\n@enduml\n", ["outer-origin", "floating note (declared residual)"])
add("no6_left_multiline_depth1", "@startuml\nstate Hold {\n  Crate --> Barrel\n}\nnote left of Hold\n  first line\n  second, longer line\nend note\n@enduml\n", ["outer-origin", "multiline note left"])
add("no6_left_leaf_beside_image", "@startuml\nstate Tank {\n  Fish --> Weed\n}\nPump --> Tank\nnote left of Pump : flow\n@enduml\n", ["outer-origin", "note on outer leaf beside image"])
add("no6_top_depth2", "@startuml\nstate Yard {\n  state Shed {\n    Rake --> Hoe\n  }\n}\nnote top of Yard : roof\n@enduml\n", ["outer-origin", "note top (declared residual)"])
add("no6_integer_canvas", "@startuml\n[*] --> Ward\nstate Ward {\n  [*] --> Bed\n  Bed --> [*]\n}\nWard --> [*]\n@enduml\n", ["outer-origin", "canvas rounding (int)(x+1)"])
# --- pass three anchors
add("pt6_note_before_nested_link", "@startuml\nstate Dam {\n  note left of Sluice : open\n  Lake --> Sluice\n}\n@enduml\n", ["pass-three", "note before anchor inside group"])
add("pt6_note_before_root_link_image", "@startuml\nnote right of Gate : shut\nstate Fort {\n  Wall --> Tower\n}\nGate --> Fort\n@enduml\n", ["pass-three", "root note before anchor, image path"])
add("pt6_note_outer_anchor_later_inner", "@startuml\nnote left of Cell : dark\nstate Keep {\n  Cell --> Hall\n}\n@enduml\n", ["pass-three", "root note, anchor first created inside group"])
add("pt6_two_notes_before", "@startuml\nnote left of Ash : grey\nnote right of Elm : tall\nAsh --> Elm\n@enduml\n", ["pass-three", "two notes before both anchors, flat"])
add("pt6_note_before_concurrent", "@startuml\nstate Hub {\n  note right of Spoke : turn\n  Rim --> Spoke\n  --\n  Axle --> Nut\n}\n@enduml\n", ["pass-three", "concurrent region"])
# --- root-path note names
add("rp6_depth3", "@startuml\nstate A1 {\n  state B1 {\n    state C1 {\n      X1 --> Y1\n      note left of Y1 : deep\n    }\n  }\n}\nZ1 --> X1\n@enduml\n", ["root-path-names", "depth 3 non-autarkic"])
add("rp6_duplicate_text", "@startuml\nstate Lab {\n  Beaker --> Flask\n  note left of Beaker : same\n  note right of Flask : same\n}\nDoor --> Beaker\n@enduml\n", ["root-path-names", "duplicate note text"])
add("rp6_root_and_group", "@startuml\nstate Den {\n  Fox --> Cub\n  note right of Cub : small\n}\nnote left of Owl : wise\nOwl --> Fox\n@enduml\n", ["root-path-names", "root note next to group note"])
add("rp6_group_note_outer_anchor", "@startuml\nRoad --> Toll\nstate Toll {\n  Booth --> Bar\n}\nstate Toll {\n  note left of Booth : pay\n}\nRoad --> Booth\n@enduml\n", ["root-path-names", "group reopened"])
add("rp6_named_floating_in_group", "@startuml\nstate Ward {\n  Nurse --> Bed\n  note \"rounds\" as N1\n}\nDesk --> Nurse\n@enduml\n", ["root-path-names", "floating named note in group (declared residual)"])
json.dump({"cases": cases}, open(D.parent / "cases.json", "w"), indent=1, ensure_ascii=False)
print(len(cases), "cases")
