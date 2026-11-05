"""Write the round-6 checker's held-out probe batch K6A (sources + cases.json). Usage: python3 -I k6a_sources.py <batch_dir>
Byte content is spelled with Python escapes; CRLF cases are written with literal CR LF line ends."""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); (D / "sources").mkdir(parents=True, exist_ok=True)
cases = []
def add(name, text, axes, valid=True, crlf=False):
    data = text.replace("\n", "\r\n") if crlf else text
    (D / "sources" / f"{name}.puml").write_bytes(data.encode("utf-8"))
    cases.append({"name": name, "axes": axes, "intended_valid": valid})
X = "\U0010FFFD"  # unmapped supplementary: forces FontUtilities.isComplexText
M = "\U0001D17B"  # Mn, unmapped everywhere on the reference host (BH/BI)

# 1. YAML/JSON display escapes, blank lines, block scalars, nesting, flow style, model JSON
Y = lambda body: "@startyaml\n" + body + "@endyaml\n"
J = lambda body: "@startjson\n" + body + "@endjson\n"
add("y8_block_inner_pipe", Y("note: |\n  example: |\n  plain line\nafter: x\n"), ["yaml", "| block whose body has a line ending in ': |'"])
add("y8_block_relative_indent", Y("note: |\n  Moss\n    Fern\n  Reed\nafter: x\n"), ["yaml", "| block, deeper-indented body line (cleanBlockStyle trims)"])
add("y8_block_spaced_pipe", Y("note:   |\n  Moss\n  Fern\nafter: x\n"), ["yaml", "| block header with several spaces after the colon"])
add("y8_block_comment_pipe", Y("note: | # tail\n  Moss\n  Fern\nafter: x\n"), ["yaml", "| block header followed by a YAML comment"])
add("y8_block_two_blocks_end", Y("a: |\n  one\nb: |\n  two\n\n\n"), ["yaml", "two | blocks, the second at the end of the document, trailing blank lines"])
add("y8_block_dash_body", Y("steps: |\n  - first\n  - second\nend: z\n"), ["yaml", "| block whose body lines start with '- '"])
add("y8_block_nested_key", Y("outer:\n  inner: |\n    Moss\n    Fern\n  side: q\n"), ["yaml", "| block under a nested key (depth 2)"])
add("y8_block_whitespace_lines", Y("note: |\n  Moss\n   \n\t\n  Fern\nafter: x\n"), ["yaml", "| block with space-only and tab-only lines (StyleExtractor drop)"])
add("y8_list_key_block", Y("items:\n  - name: |\n      Moss\n"), ["yaml", "list item key with | block (Java YamlParser wip3)"], valid=False)
add("y8_folded", Y("note: >\n  Moss\n  Fern\nafter: x\n"), ["yaml", "> folded block (Java YamlParser wip4)"], valid=False)
add("y8_escape_backslash_pairs", Y('path: "C:\\\\dock\\\\new"\nplain: a\\\\b\n'), ["yaml", "doubled backslashes quoted and plain; \\\\ before n"])
add("y8_escape_unknown_pairs", Y("k: a\\qb\\x41\\u0042\n"), ["yaml", "unknown escape pairs kept"])
add("y8_escape_trailing", Y("k: abc\\\nm: \\\\\n"), ["yaml", "trailing single backslash; value of one doubled backslash"])
add("y8_escape_key_tab", Y("a\\tb: v\nwide key\\tz: w\n"), ["yaml", "\\t in keys (bold tab stops)"])
add("y8_real_tab_value", Y("k: a\tb\nm: \tlead\n"), ["yaml", "literal TAB inside values (YamlLine replaces TAB by four spaces)"])
add("y8_escape_n_then_t", Y('k: "a\\n\\tb\\n\\t\\tc"\n'), ["yaml", "\\t at line starts after \\n"])
add("y8_align_mixed", Y("k: one\\ltwo\\rthree\nm: x\\ry\\lz\n"), ["yaml", "mixed \\l and \\r natural alignment, last wins"])
add("y8_flow_seq_escapes", Y("k: [a\\nb, \"c\\\\d\", e\\tf]\n"), ["yaml", "flow sequence items with escapes"])
add("y8_deep_nested_escape", Y("l1:\n  l2:\n    l3\\nkey: v\\tw\n    other: \\\\\n"), ["yaml", "depth 3, \\n in key, \\t in value"])
add("y8_whitespace_values", Y('sp: "   "\ntb: "\\t"\nmix: " a "\n'), ["yaml", "space-only, tab-escape-only and padded values"])
add("y8_blank_lines_everywhere", Y("\n  \n\t\na: 1\n\n   \nb:\n\n  c: 2\n\n"), ["yaml", "leading, interleaved and nested blank and whitespace-only lines"])
add("y8_crlf", Y("k: Moss\\nFern\nnote: |\n  Reed\n  Rush\nz: end\n"), ["yaml", "CRLF line ends throughout"], crlf=True)
add("y8_block_inner_pipe_end", Y("first: x\nnote: |\n  Moss\n  ratio: |\n"), ["yaml", "| block whose last body line ends in ': |', end of document"])
add("y8_block_inner_pipe_nested", Y("outer:\n  inner: |\n    a: |\n    b\n  side: q\n"), ["yaml", "nested | block whose body has a line ending in ': |'"])
add("s8_chr10_seq", "@startuml\nAlice -> Bob : Moss%chr(10)Fern\n@enduml\n", ["real LF", "%chr(10) inside a sequence message"])
add("s8_chr13_seq", "@startuml\nAlice -> Bob : Moss%chr(13)Fern\n@enduml\n", ["real CR", "%chr(13) inside a sequence message"])
add("s8_chr13_class", "@startuml\nclass \"Moss%chr(13)Fern\" as V\n@enduml\n", ["real CR", "%chr(13) inside a class display name"])
add("y8_tabsize4", Y("skinparam tabSize 4\nk: \"a\\tb\"\nm: wide\\tz\n"), ["yaml", "skinparam tabSize 4 (AtomText tab stop from FontConfiguration.tabSize)"])
add("j8_tabsize4", J('skinparam tabSize 4\n{"k": "a\\tb", "w": "wide\\tz"}\n'), ["json", "skinparam tabSize 4 with decoded TAB"])
add("j8_decoded_tab", J('{"k": "a\\tb", "w": "WWWWWWWWWWWW\\tx"}\n'), ["json", "decoded TAB (real U+0009), tab after a token wider than one stop"])
add("j8_decoded_cr", J('{"k": "a\\rb", "z": 1}\n'), ["json", "decoded CR (real U+000D) mid-line"])
add("j8_decoded_crlf_tail", J('{"k": "abc\\r\\n", "z": 1}\n'), ["json", "decoded CR LF at the end of a value"])
add("j8_decoded_backslash", J('{"k": "a\\\\b", "n": "a\\\\nb", "d": "a\\\\\\\\nb"}\n'), ["json", "decoded backslash pairs: \\b unknown, \\n newline, doubled backslash then n"])
add("j8_unicode_lf", J('{"k": "a\\u000Ab", "t": "c\\u0009d"}\n'), ["json", "\\u000A and \\u0009 escapes"])
add("j8_tab_bold_key", J('{"a\\tb": 1, "wide key\\tz": 2}\n'), ["json", "decoded TAB in keys (bold)"])
add("j8_empty_space", J('{"e": "", "s": "  ", "p": " x "}\n'), ["json", "empty, space-only, padded values"])
add("j8_array_escapes", J('["x\\\\ny", "p\\tq", ["r\\\\rs", "t"]]\n'), ["json", "top-level array with escapes, nested array"])
add("j8_nested_uneven", J('{"a": {"k": "one\\\\ntwo\\\\nthree", "m": "x"}, "b": ["u\\\\nv", "w"], "c": 1}\n'), ["json", "nested boxes with uneven multi-line rows"])
add("j8_highlight_tab_key", J('#highlight "a\\tb"\n{"a\\tb": "v\\\\nw", "z": 1}\n'), ["json", "highlighted row whose key has a TAB and value two lines"])
add("j8_crlf", J('{\n  "k": "Moss\\\\nFern",\n  "z": [1, 2]\n}\n'), ["json", "CRLF line ends"], crlf=True)
add("m8_json_class", "@startuml\njson Dock {\n  \"k\": \"Moss\\\\nFern\",\n  \"t\": \"a\\tb\"\n}\n@enduml\n", ["model json", "json element in a class diagram: escapes"])
add("m8_json_usecase", "@startuml\nallowmixing\nactor Pilot\njson Log {\n  \"when\": \"dawn\\\\rdusk\",\n  \"list\": [\"x\\\\ny\", \"z\"]\n}\n@enduml\n", ["model json", "json element mixed into a usecase diagram"])
add("m8_json_crlf", "@startuml\njson Dock {\n  \"k\": \"a\\\\\\\\b\"\n}\n@enduml\n", ["model json", "CRLF, doubled backslash"], crlf=True)
add("m8_class_crlf_label", "@startuml\nclass Pier {\n  +moor()\n  -rope : int\n}\nnote right of Pier : tidal\n@enduml\n", ["crlf", "class diagram with CRLF line ends (real CR never measured)"], crlf=True)
add("m8_seq_crlf", "@startuml\nAlice -> Bob : ping\nBob --> Alice : pong\n@enduml\n", ["crlf", "sequence with CRLF line ends"], crlf=True)

# 2. Unmapped marks
cls = lambda txt, pre="": "@startuml\n" + pre + f'class "{txt}" as V\n@enduml\n'
add("mk8_u17_1ad0", cls("Vault \u1AD0" + X), ["mark", "U+1AD0 Mn only from Unicode 17 (Cn in HarfBuzz 10.0.1's UCD 16): discriminates the loaded HarfBuzz"])
add("mk8_u17_1e6e3", cls("Vault \U0001E6E3"), ["mark", "U+1E6E3 Mn only from Unicode 17, supplementary"])
add("mk8_u18_05c8", cls("Vault \u05C8"), ["mark", "U+05C8 Mn only from Unicode 18 (HarfBuzz 14.6+), Hebrew block (complex)"])
add("mk8_u16_113bb", cls("Vault \U000113BB"), ["mark", "U+113BB Tulu-Tigalari Mn (Unicode 16), new block"])
add("mk8_u16_10d69", cls("Vault \U00010D69"), ["mark", "U+10D69 Garay Mn (Unicode 16)"])
add("mk8_u16_1612d", cls("Vault \U0001612D"), ["mark", "U+1612D Gurung Khema Mn (Unicode 16)"])
add("mk8_me_1abe", cls("Vault \u1ABE" + X), ["mark", "U+1ABE Me (enclosing): not synthesized as MARK"])
add("mk8_mc_11f03", cls("Vault \U00011F03"), ["mark", "U+11F03 Kawi Mc: not zeroed"])
add("mk8_helv_hebrew", cls("Vault \u05D0" + M, "skinparam defaultFontName Helvetica\n"), ["mark", "Helvetica primary; Hebrew base from the Lucida Grande slot; mark alone in the Helvetica buffer"])
add("mk8_helv_hebrew_ctrl", cls("Vault \u05D0", "skinparam defaultFontName Helvetica\n"), ["mark", "control for mk8_helv_hebrew without the mark"])
add("mk8_lucida_arabic", cls("Vault \u0627" + M), ["mark", "Lucida Grande primary; Arabic base from the Geeza Pro slot; mark alone in its buffer"])
add("mk8_lucida_arabic_ctrl", cls("Vault \u0627"), ["mark", "control for mk8_lucida_arabic"])
add("mk8_lucida_syriac", cls("Vault \u0710" + M), ["mark", "Syriac base unmapped everywhere (slot 0, same buffer): arabic shaper fallback zeroes"])
add("mk8_helv_hebrew_pres", cls("Vault \uFB1D" + M, "skinparam defaultFontName Helvetica\n"), ["mark", "Helvetica; Hebrew FB1D unmapped everywhere (same buffer)"])
add("mk8_lucida_devanagari", cls("Vault \u0915" + M), ["mark", "Devanagari base (Kohinoor slot): indic shaper, never zeroed"])
add("mk8_lucida_devanagari_ctrl", cls("Vault \u0915"), ["mark", "control for mk8_lucida_devanagari"])
add("mk8_cjk_base", cls("Vault \u4E00" + M), ["mark", "Han base (PingFang slot): default shaper zeroes regardless"])
add("mk8_cjk_base_ctrl", cls("Vault \u4E00"), ["mark", "control for mk8_cjk_base"])
add("mk8_hebrew_lead_mark", cls(M + "\uFB1D Vault"), ["mark", "leading mark in a Hebrew run, no base"])
add("mk8_menlo_hebrew", cls("Vault \u05D0" + M, "skinparam defaultFontName Monospaced\n"), ["mark", "Monospaced (Menlo) maps Hebrew itself: same buffer"])
add("mk8_seq_arabic", "@startuml\nAlice -> Bob : Vault \u0627" + M + "\n@enduml\n", ["mark", "sequence message, Arabic base from another slot"])
for fam, pre in [("sans", ""), ("helv", "skinparam defaultFontName Helvetica\n"), ("mono", "skinparam defaultFontName Monospaced\n"),
                 ("serif", "skinparam defaultFontName Serif\n"), ("verdana", "skinparam defaultFontName Verdana\n"),
                 ("arial", "skinparam defaultFontName Arial\n"), ("couriernew", "skinparam defaultFontName \"Courier New\"\n")]:
    for size in (9, 23):
        add(f"mk8_{fam}_{size}", cls("Vault " + M + "q\U0001E5EE", pre + f"skinparam defaultFontSize {size}\n"),
            ["mark", f"family {fam}", f"size {size}", "two unmapped Mn after bases"])
add("mk8_sans_bold_23", cls("<b>Vault " + M + "q\U0001E5EE</b>", "skinparam defaultFontSize 23\n"), ["mark", "Lucida Grande Bold", "size 23"])

# 3. PingFang-asset and user-font scalars never probed (same ranges)
for cp, why in [(0x31B8, "PingFang asset complex-only 31B8-31BF"), (0x324A, "PingFang asset complex-only 3248-324F"),
                (0xFE22, "PingFang asset complex-only FE20-FE23"), (0x1F105, "PingFang asset 1F101-1F10A"),
                (0x327E, "simple-path only on Monkey Man 327E"), (0xF830, "simple-path F828-F83B"), (0xFA2E, "simple-path FA2E-FA2F"),
                (0xF881, "simple-path F880-F881 (LiHeiPro)"), (0x218A, "user font 218A"), (0xF851, "user font F851-F85E"),
                (0xF8C0, "user font F8B7-F8FE"), (0xF8FE, "user font F8FE")]:
    ch = chr(cp)
    if cp < 0x10000:
        add(f"fe8_{cp:04x}_simple", cls("Vault " + ch), ["font environment", why, "simple path"])
    add(f"fe8_{cp:04x}_complex", cls("Vault " + ch + X), ["font environment", why, "complex path"])

# 4. Canvas (SvgGraphics.ensureVisible): picks from canvas_pool.py. Outer: base ceil vs tip (int)(x+1) differ exactly on integral
#    widths ("int"); every outer and root height in the pool is integral; every root width fractional (merge ceil = tip + 1).
CT = {
 "outer": lambda a, b, c: f"@startuml\n[*] --> {a}\nstate {a} {{\n  [*] --> {b}\n  {b} --> {c}\n  {c} --> [*]\n}}\n{a} --> [*]\n@enduml\n",
 "root": lambda a, b, c: f"@startuml\nstate {a} {{\n  {b} --> {c}\n  note right of {c} : calm\n}}\nnote left of Gull : drift\nGull --> {b}\n@enduml\n",
 "rootlate": lambda a, b, c: f"@startuml\nnote left of {c} : busy\nstate {a} {{\n  {b} --> {c}\n}}\n@enduml\n",
 "flat": lambda a, b, c: f"@startuml\n[*] --> {a}\n{a} --> {b} : go\n{b} --> {c}\n{c} --> [*]\n@enduml\n",
}
for kind, names, what in [
    ("outer", ("Pier", "Jetty", "Mm"), "integral width and height"), ("outer", ("Harbour", "Ko", "Pier"), "integral width and height"),
    ("outer", ("Mm", "Pier", "Il"), "integral width and height"), ("outer", ("Dock", "Mm", "Harbour"), "fractional width, integral height"),
    ("outer", ("Buoy", "Mm", "Lighthouse"), "fractional width, integral height"),
    ("root", ("Pier", "Jetty", "Mm"), "fractional width, integral height"), ("root", ("Quay", "Dock", "Harbour"), "fractional width, integral height"),
    ("root", ("Wharf", "Anchorage", "Pier"), "fractional width, integral height"),
    ("rootlate", ("Quay", "Mooring", "Wharf"), "fractional width, integral height"), ("rootlate", ("Mooring", "Pier", "Ko"), "fractional width, integral height"),
    ("flat", ("Mooring", "Harbour", "Slipway"), "flat state path"), ("flat", ("Ko", "Jetty", "Mm"), "flat state path")]:
    add(f"cv8_{kind}_{'_'.join(n.lower() for n in names)}", CT[kind](*names), ["canvas", f"{kind} path", what])
add("cv8_class_note", "@startuml\nclass Mm\nnote right of Mm : Il\n@enduml\n", ["canvas", "class family (svek)", "short labels"])
add("cv8_usecase", "@startuml\nactor Pilot\nusecase (Moor) as M\nPilot --> M\n@enduml\n", ["canvas", "usecase family"])
add("cv8_component", "@startuml\n[Pump] --> [Valve]\n@enduml\n", ["canvas", "component family"])
add("cv8_activity", "@startuml\nstart\n:Moor;\nstop\n@enduml\n", ["canvas", "activity family"])

json.dump({"batch": "K6A", "cases": cases}, open(D / "cases.json", "w"), indent=1, ensure_ascii=False)
print(len(cases), "cases")
