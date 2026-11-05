"""Write the round-7 maker held-out batch L7A (fresh probes, labels renamed from BJ).
Usage: python3 -I l7a_sources.py <out dir>  -> <name>.puml + cases.json (name, intended_valid, axes)"""
import json, pathlib, sys
out = pathlib.Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
cases = []
def case(name, body, axes, valid=True):
    (out / f"{name}.puml").write_bytes(body.encode("utf-8"))
    cases.append(dict(name=name, intended_valid=valid, axes=axes))
def cls(label, font=None):
    sp = f"skinparam defaultFontName {font}\n" if font else ""
    return f'@startuml\n{sp}class "{label}" as V\n@enduml\n'
A = "\U0010FFFD"; M = "\U0001D17B"
# Unicode 17/18 nonspacing marks (HarfBuzz 15.0.0 UCD) in default-shaped runs, complex via an astral scalar.
for cp, word in [(0x1AD5, "Harbor"), (0x10EFA, "Quay"), (0x11B60, "Pier"), (0x0B53, "Dock"), (0x1D127, "Slip"), (0x10ECB, "Mole")]:
    case(f"mk9_u{'17' if cp in (0x1AD5, 0x10EFA, 0x11B60) else '18'}_{cp:x}", cls(f"{word} {A}{chr(cp)}"), ["mark", "ucd18", "default-run"])
# U+180F is default-ignorable in 15.0.0: hidden even in a Thai run.
case("mk9_180f_thai", cls(f"Reef ก᠏"), ["mark", "default-ignorable", "thai-run"])
case("mk9_180f_thai_ctrl", cls(f"Reef ก"), ["control"])
# Buffers: a slot-0 space is a base inside a Hebrew/Arabic run.
for font, tag in [(None, "lucida"), ("Helvetica", "helv"), ("Verdana", "verdana"), ("Arial", "arial"), ("Courier New", "couriernew"), ("Monospaced", "mono")]:
    case(f"mk9_{tag}_hebrew_space", cls(f"Reef א {M}z", font), ["mark", "buffer", "hebrew-run", tag])
    case(f"mk9_{tag}_hebrew_space_ctrl", cls(f"Reef א z", font), ["control", tag])
case("mk9_lucida_arabic_space", cls(f"Reef ا {M}z"), ["mark", "buffer", "arabic-run"])
case("mk9_lucida_arabic_space_ctrl", cls(f"Reef ا z"), ["control"])
case("mk9_helv_syriac", cls(f"Reef ܐ{M}", "Helvetica"), ["mark", "buffer", "syriac-unmapped-base"])
case("mk9_helv_syriac_ctrl", cls(f"Reef ܐ", "Helvetica"), ["control"])
case("mk9_lucida_thai_base", cls(f"Reef ก{M}"), ["mark", "thai-run"])
case("mk9_lucida_deva_base", cls(f"Reef क{M}"), ["mark", "indic-run"])
case("mk9_lucida_deva_ctrl", cls(f"Reef क"), ["control"])
case("mk9_lucida_hebrew_wj", cls(f"Reef א⁠{M}"), ["mark", "default-ignorable-between"])
case("mk9_lucida_hebrew_hiriq", cls(f"Reef אִ{M}"), ["mark", "mapped-mark-between"])
case("mk9_lucida_hebrew_hiriq_ctrl", cls(f"Reef אִ"), ["control"])
case("mk9_hangul_space", cls(f"Reef {A}가 {M}z"), ["mark", "buffer", "hangul-run", "hb12.2-fallback"])
case("mk9_hangul_space_ctrl", cls(f"Reef {A}가 z"), ["control"])
case("mk9_hangul_base", cls(f"Reef {A}가{M}"), ["mark", "buffer", "hangul-other-slot"])
case("mk9_seq_hebrew", f"@startuml\nAlice -> Bob : Reef א{M}\n@enduml\n", ["mark", "sequence"])
case("mk9_seq_hebrew_ctrl", f"@startuml\nAlice -> Bob : Reef א\n@enduml\n", ["control", "sequence"])
# YAML blocks (YamlParser.getBlockStyleString) and lines (YamlLine).
case("y9_block_depth3", "@startyaml\nharbor:\n  fleet:\n    log: |\n      tide: |\n      ebb line\n    crew: nine\n@endyaml\n", ["yaml", "block", "depth3"])
case("y9_block_text_dedent", "@startyaml\nnote: |\n  Moss\nloose words\nafter: x\n@endyaml\n", ["yaml", "block", "dedented-text"])
case("y9_block_hash_lines", "@startyaml\nnote: |\n  Moss # kept\n  # hash line\nafter: x\n@endyaml\n", ["yaml", "block", "comments"])
case("y9_block_tab_inside", "@startyaml\nnote: |\n  a\tb\nafter: x\n@endyaml\n", ["yaml", "block", "tab"])
case("y9_block_quoted_key", "@startyaml\n\"odd: key\": |\n  Reed\n  Rush\nnext: y\n@endyaml\n", ["yaml", "block", "quoted-key"])
case("y9_tab_indent", "@startyaml\nouter:\n\tinner: v\n\tside: w\n@endyaml\n", ["yaml", "tab", "indent"])
case("y9_tab_key", "@startyaml\nk\tey: v\tw\n@endyaml\n", ["yaml", "tab", "key"])
case("y9_flow_seq_quotes", "@startyaml\nk: ['a,b', \"c\\\"d\", plain , ]\n@endyaml\n", ["yaml", "flow-sequence"])
case("y9_flow_seq_empty", "@startyaml\nk: []\nz: q\n@endyaml\n", ["yaml", "flow-sequence", "empty"])
case("y9_flow_seq_number", "@startyaml\nk: [1.50, 0x10, true]\n@endyaml\n", ["yaml", "flow-sequence", "strings"])
# StyleExtractor preamble.
case("j9_skinparam_block", "@startjson\nskinparam {\n  RoundCorner 20\n}\n{\"k\": 1}\n@endjson\n", ["json", "skinparam", "block"])
case("y9_hide_then_data", "@startyaml\nhide empty\nk: v\n@endyaml\n", ["yaml", "preamble"])
case("y9_skinparam_after_data", "@startyaml\nk: v\nskinparam tabSize 4\n@endyaml\n", ["yaml", "skinparam", "after-data"], valid=False)
# CR in character data (ToStream).
case("j9_cr_key", "@startjson\n{\"a\\rb\": 1, \"c\": \"d\\re\"}\n@endjson\n", ["json", "cr", "key"])
(out / "cases.json").write_text(json.dumps(cases, indent=1, ensure_ascii=False) + "\n")
print(len(cases), "cases")
