"""Write the m5a probe sources (held out: not in any batch). Usage: python3 -I m5a_sources.py <dir>"""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); (D / "sources").mkdir(parents=True, exist_ok=True)
U = "@startuml\n{}\n@enduml\n"
def cls(text, pre=""):
    return U.format((pre + "\n" if pre else "") + f'class "{text}" as V')
fam = lambda f: f"skinparam defaultFontName {f}"
C = []
def add(name, src, axes, valid=True):
    C.append((name, src, axes, valid))
# obligation 1/5: outer and root-path notes
add("no5_left_note_renamed", U.format("state Forge {\n  Ore --> Ingot\n}\nnote left of Forge : glowing hot"), ["note left of an image at root, renamed"])
add("no5_three_notes_image", U.format("state Mint {\n  Die --> Coin\n}\nnote left of Mint : press\nnote right of Mint : stamp\nnote left of Mint : polish"), ["three notes on one image, two on the left"])
add("no5_left_note_start_top", U.format("[*] --> Kiln\nstate Kiln {\n  Clay --> Ware\n}\nnote left of Kiln : hot"), ["note left of an image, start ellipse on the top rank"])
add("no5_left_note_depth2_region", U.format("state Yard {\n  state Shed {\n    Saw --> Plank\n  }\n  note left of Shed : dusty\n}"), ["note left of a depth-2 image inside a region"])
add("no5_right_note_nested_image", U.format("state Hub {\n  state Spoke {\n    Rim --> Tire\n  }\n}\nnote right of Hub : round"), ["note right of an image holding an image"])
add("no5_note_top_image", U.format("state Pier {\n  Rope --> Knot\n}\nnote top of Pier : salty"), ["note above an image"])
add("no5_note_bottom_image", U.format("state Dune {\n  Sand --> Glass\n}\nnote bottom of Dune : hot sand"), ["note below an image"])
add("no5_left_note_multiline_renamed", U.format("state Loom {\n  Warp --> Weft\n}\nnote left of Loom\n  woven\n  tight\n  and long\nend note"), ["multi-line note left of an image, three lines"])
add("rp5_root_note_depth2", U.format("state Hall {\n  state Wing {\n    Larch --> Cedar\n    note right of Larch : resin\n  }\n}\nGate --> Larch"), ["root cluster path, note at depth 2"])
add("rp5_two_groups_sides", U.format("state East {\n  Oak --> Elm\n  note left of Oak : bark\n}\nstate West {\n  Fir --> Yew\n  note right of Yew : sap\n}\nStart --> Oak\nElm --> Fir\nnote right of Start : go"), ["root cluster path, notes in two groups and at root"])
# obligation 2: unmapped scalars, several blocks
for name, cp, axis in [("0378", 0x0378, "BMP unassigned (Greek block)"), ("fdd0", 0xFDD0, "BMP noncharacter"),
                       ("e123", 0xE123, "BMP private use"), ("12000", 0x12000, "Cuneiform, outside the cascade"),
                       ("2fffd", 0x2FFFD, "plane-2 unassigned"), ("1bca0", 0x1BCA0, "shorthand format control, not hidden"),
                       ("e0100", 0xE0100, "variation selector 17 (hidden)"), ("1d173", 0x1D173, "musical formatting (hidden)"),
                       ("20000", 0x20000, "CJK Ext B, outside the Lucida cascade"), ("a500", 0xA500, "Vai, CoreText substitution only")]:
    add(f"w5_sans_{name}", cls(f"Vault {chr(cp)}"), [f"U+{cp:04X} {axis}", "SansSerif"])
add("w5_sans_a500_complex", cls(f"Vault {chr(0xA500)}{chr(0x10FFFD)}"), ["U+A500 next to U+10FFFD (complex text)", "SansSerif"])
add("w5_sans_bold18_10fffe", cls(f"Vault {chr(0x10FFFE)}", "skinparam classFontSize 18\nskinparam classFontStyle bold"), ["U+10FFFE", "SansSerif bold 18"])
for f, key in [("Helvetica", "helvetica"), ("Verdana", "verdana"), ("Courier New", "couriernew"), ("Arial", "arial"), ("Times New Roman", "times"), ("Monospaced", "mono")]:
    add(f"w5_{key}_ctl", cls("Ledger ", fam(f)), [f"control 'Ledger ' {f}"])
    add(f"w5_{key}_10fffd", cls(f"Ledger {chr(0x10FFFD)}", fam(f)), [f"U+10FFFD {f}"])
    add(f"w5_{key}_10f800", cls(f"Ledger {chr(0x10F800)}", fam(f)), [f"U+10F800 (carrier range, escaped) {f}"])
add("w5_helvetica_bold_10fffd", cls(f"Ledger {chr(0x10FFFD)}", fam("Helvetica") + "\nskinparam classFontStyle bold"), ["U+10FFFD Helvetica bold"])
add("w5_mono_e0041", cls(f"Vault {chr(0xE0041)}", fam("Monospaced")), ["U+E0041 tag, Monospaced (hidden)"])
add("w5_times_12000", cls(f"Vault {chr(0x12000)}", fam("Times New Roman")), ["U+12000 Times New Roman"])
add("w5_arial_10f7ff", cls(f"Vault {chr(0x10F7FF)}", fam("Arial")), ["U+10F7FF (escape scalar) Arial"])
# obligation 4: post-entry decoders
add("dc5_yaml_key_escape", "@startyaml\n\"Pass \\U0010FFFD\": \"Gate \\U0010F7FF\"\n@endyaml\n", ["YAML double-quoted key and value with \\U escapes"])
add("dc5_yaml_list_escape", "@startyaml\nreeds:\n  - \"Reed \\U0010FFFD\"\n  - plain\n@endyaml\n", ["YAML list item with \\U escape"])
add("dc5_yaml_flow_escape", "@startyaml\nk: [\"Reed\\U0010FFFD\", b]\n@endyaml\n", ["YAML flow sequence with \\U escape (residue)"])
add("dc5_json_nested", "@startjson\n{\"outer\": {\"list\": [\"Reed \\uDBFF\\uDFFD\", {\"\\uDBFF\\uDC00\": true}]}}\n@endjson\n", ["JSON surrogate-pair escapes nested in arrays and as a key"])
add("dc5_tim_json_member", U.format("!$j = { \"a\": { \"b\": \"Reed \\uDBFF\\uDFFD\" } }\nclass \"$j.a.b\" as J"), ["TIM JSON nested member with a pair escape"])
add("dc5_tim_json_array", U.format("!$j = [\"x\", \"Reed \\uDBFF\\uDC00\"]\nclass \"$j[1]\" as J"), ["TIM JSON array index with a pair escape"])
# comment '?' rule: an unpaired unit only in an entity code
add("cm_unit_class_code", U.format("!$u = %substr(\"\U0001F680\", 0, 1)\nclass \"Shown\" as Q$u"), ["unpaired unit in a class code only"], valid=False)
add("cm_unit_link_code", U.format("!$u = %substr(\"\U0001F680\", 0, 1)\nclass A\nclass \"Shown\" as B$u\nA --> B$u"), ["unpaired unit in a link endpoint code"], valid=False)
add("cm_unit_package_code", U.format("!$u = %substr(\"\U0001F680\", 0, 1)\npackage \"Shown\" as P$u {\n  class C\n}"), ["unpaired unit in a package code"], valid=False)
for name, src, axes, valid in C:
    (D / "sources" / f"{name}.puml").write_text(src, encoding="utf-8")
json.dump([{"name": n, "axes": a, "intended_valid": v} for n, _, a, v in C], open(D / "cases.json", "w"), indent=1, ensure_ascii=False)
print(len(C), "cases")
