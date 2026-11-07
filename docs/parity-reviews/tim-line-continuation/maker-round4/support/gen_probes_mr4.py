"""Maker round-4 held-out probes (batch MR4) for T14.22 round 4. Written before any Java result for these inputs.
intended_valid is the maker's Java prediction from the pinned source trace (TLineType/TContext#addPlain, UmlSource
#loadInternal/#isEmpty, PSystemWelcomeFactory, PSystemCommandFactory, CodeIteratorProcedure/ReturnFunction,
StartUtils, Character#isWhitespace, ReadLineReader#readLine, DiagramDetector, UncommentReadLine,
TContext#executeInclude). 'welcome' predicts Java's exit-0 Welcome page (recorded as intended_valid=True with the
axis 'java:welcome'); 'empty' predicts the exit-200 'Empty description' page (intended_valid=False).
Usage: python3 -I gen_probes_mr4.py OUT_DIR"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
S = "@startuml\n"; E = "@enduml\n"
T = "Kea -> Tui : x %chr(92)\n"
ONE = "Kea -> Tui : one\n"; TWO = "Tui -> Kea : two\n"
RES = {
 "mr4_w_bom_all.iuml": "\ufeff@startuml\n\ufeffKea -> Tui : b1\n\ufeff@enduml\n",
 "mr4_r_bom_directive.iuml": "\ufeff!$who = \"Moa\"\n\ufeffKea -> $who : hi\n",
 "mr4_r_endash.iuml": "Kea \u2013> Tui : a \u2013 b\n",
 "mr4_w_endash.iuml": "@startuml\nKea \u2013> Tui : w\n@enduml\n",
 "mr4_w_half.iuml": "\u00bd @startuml\n\u00bd Kea -> Tui : half\n\u00bd @enduml\n",
 "mr4_w_roman.iuml": "\u2160 @startuml\n\u2160 Kea -> Tui : roman\n\u2160 @enduml\n",
 "mr4_w_supp.iuml": "\U0001d400@startuml\n\U0001d400Kea -> Tui : supp\n\U0001d400@enduml\n",
 "mr4_w_two.iuml": "@startuml\nKea -> Tui : first\n@enduml\n@startuml\nKea -> Tui : second\n@enduml\n",
 "mr4_r_plain.iuml": "Kea -> Tui : plain\n",
 "mr4_sub_bom.iuml": "\ufeff!startsub PART\nKea -> Tui : sub\n!endsub\n",
}
C = []
def add(name, src, pred, axes):
    C.append(dict(name=name, src=src, pred=pred, axes=axes))
# Obligation 1: !pragma is a PLAIN line of the TIM list; Rust-only lines are not.
add("mr4_pragma_svek_start", S + "!pragma svek_trace on\n" + "Tui -> Kea : z %chr(92)\n" + E, True, ["pragma", "position:start", "count:3"])
add("mr4_pragma_two_then_tail", S + "!pragma a b\n!pragma c d\n" + T + E, True, ["pragma", "two pragmas", "count:4"])
add("mr4_pragma_middle_vertical", S + T + "!pragma useVerticalIf on\n" + "Tui -> Kea : y\n" + E, True, ["pragma", "position:middle", "join into pragma text"])
add("mr4_pragma_inactive_tail", S + "!if %false()\n!pragma teoz true\n!endif\n" + T + E, "welcome", ["pragma", "inactive branch", "count:2"])
add("mr4_pragma_from_procedure", S + "!procedure $p()\n!pragma layout smetana\n!endprocedure\n$p()\n" + T + E, True, ["pragma", "procedure output", "count:3"])
add("mr4_pragma_variable_value", S + "!$v = \"smetana\"\n!pragma layout $v\n" + T + E, True, ["pragma", "variable", "count:3"])
add("mr4_two_option_handwritten_tail", S + "!option handwritten true\n" + T + E, "welcome", ["rust-only line", "option handwritten", "count:2"])
add("mr4_two_dump_tail", S + "!dump_memory\n" + T + E, "welcome", ["rust-only line", "dump_memory", "count:2"])
add("mr4_two_option_nojoin", S + "!option handwritten true\n" + E, "welcome", ["rust-only line", "no continuation", "count:2"])
add("mr4_two_comment_and_define_nojoin", S + "' c\n!define X y\n" + E, "welcome", ["no continuation", "count:2"])
add("mr4_theme_unshipped_tail", S + "!theme mono\n" + T + E, True, ["theme", "java-bundled theme Rust does not ship", "count>2"])
add("mr4_theme_shipped_tail", S + "!theme cerulean\n" + T + E, True, ["theme", "shipped theme", "count>2"])
# Obligation 2: one-line sources; end lines swallowed by a pending definition.
add("mr4_one_at_end_in_procdef", S + "!procedure $p()\nKea -> Tui : p\n@enduml\n!endprocedure\n$p()\n" + E, "empty", ["one-line", "@enduml in !procedure definition"])
add("mr4_one_bs_end_in_function", S + "!function $f()\n!return 1\n\\enduml\n!endfunction\n" + E, "empty", ["one-line", "\\enduml in !function definition"])
add("mr4_one_end_in_definelong", S + "!definelong D()\nKea -> Tui : d\n\\enduml\n!enddefinelong\nD()\n" + E, "empty", ["one-line", "\\enduml in !definelong"])
add("mr4_two_content_then_procdef_end", S + "Kea -> Tui : before\n!procedure $p()\n\\enduml\n!endprocedure\n" + E, "welcome", ["two-line", "end in definition after content"])
add("mr4_one_end_continued_after_directive", S + "!$x = 1\n@enduml \\\n", "empty", ["one-line", "end line continued", "directive before"])
add("mr4_terminated_def_then_bs_end", S + "!procedure $p()\nKea -> Tui : p\n!endprocedure\n$p()\n\\enduml\n", True, ["control", "terminated definition", "\\enduml"])
# Obligation 3: Character#isWhitespace before an end directive.
for cp, nm, pred in [(0x1C, "fs", True), (0x1D, "gs", True), (0x1E, "rs", True), (0x3000, "ideographic", True),
                     (0x2028, "line_separator", True), (0x2001, "em_quad", True), (0x200B, "zwsp", False),
                     (0x2007, "figure_space", False), (0x180E, "mongolian_vs", False)]:
    add(f"mr4_ws_{nm}_before_bs_end", S + ONE + chr(cp) + "\\enduml\n" + TWO + E, pred, ["whitespace", f"U+{cp:04X}", "\\enduml"])
add("mr4_ws_gs_before_at_end", S + ONE + "\u001d@enduml\n" + TWO + E, True, ["whitespace", "U+001D", "@enduml (not Pipe's end)"])
add("mr4_ws_nel_before_at_end", S + ONE + "\u0085@enduml\n" + TWO + E, False, ["whitespace", "U+0085", "@enduml"])
# Obligation 4: ReadLineReader, isLetterOrDigit, include suffix.
add("mr4_inc_bom_every_line", S + "!include mr4_w_bom_all.iuml\n" + E, True, ["include", "BOM on every line", "wrapped"])
add("mr4_inc_bom_directive_raw", S + "!include mr4_r_bom_directive.iuml\n" + E, True, ["include", "BOM before a directive", "raw"])
add("mr4_inc_endash_raw", S + "!include mr4_r_endash.iuml\n" + E, True, ["include", "U+2013", "raw"])
add("mr4_inc_endash_wrapped", S + "!include mr4_w_endash.iuml\n" + E, True, ["include", "U+2013", "wrapped"])
add("mr4_inc_header_half", S + "!include mr4_w_half.iuml\n" + E, True, ["include", "header U+00BD (No)"])
add("mr4_inc_header_roman", S + "!include mr4_w_roman.iuml\n" + E, True, ["include", "header U+2160 (Nl)"])
add("mr4_inc_header_supplementary", S + "!include mr4_w_supp.iuml\n" + E, True, ["include", "header U+1D400 (supplementary letter)"])
add("mr4_inc_bang1", S + "!include mr4_w_two.iuml!1\n" + E, True, ["include", "!1 suffix", "wrapped"])
add("mr4_inc_bang_name_raw", S + "!include mr4_r_plain.iuml!x\n" + E, True, ["include", "!x suffix", "raw"])
add("mr4_includesub_bom", S + "!includesub mr4_sub_bom.iuml!PART\n" + E, True, ["includesub", "BOM before !startsub"])
for c in C:
    (out / f"{c['name']}.puml").write_text(c["src"], encoding="utf-8")
for path, text in RES.items():
    (out / path).write_text(text, encoding="utf-8")
cases = [dict(name=c["name"], intended_valid=(c["pred"] is True or c["pred"] == "welcome"),
              java_prediction={True: "valid", False: "error", "welcome": "welcome", "empty": "empty-description"}[c["pred"]],
              axes=c["axes"] + (["java:welcome"] if c["pred"] == "welcome" else []), crlf=False) for c in C]
(out / "cases.json").write_text(json.dumps(cases, indent=1, ensure_ascii=False))
(out / "resources.json").write_text(json.dumps(sorted(RES), indent=1))
print(len(cases), len(RES))
