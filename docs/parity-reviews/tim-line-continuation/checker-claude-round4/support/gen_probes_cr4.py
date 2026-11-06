"""Checker round-4 held-out probes (batch CR4) for T14.22 round 4 (maker tip fa3f95010, code 9ed52f435).
Written before any Java result for these inputs exists. intended_valid / java_prediction is the checker's prediction
from the pinned Java source (71806a237): TLineType (directive regexes use [%s] = [\\s\\u00A0], ASCII \\s),
TContext#addPlain/executeInclude/includesub, CodeIterator{Sub,Procedure,ReturnFunction,Foreach,While,If},
UmlSource#loadInternal/getTotalLineCount/isEmpty (COMMENT_LINE, StringUtils#trin), PSystemWelcomeFactory,
PSystemCommandFactory#createSystem, PSystemBuilder fall-through, StartUtils (isEndDirective, isExit,
beforeStartUml/isWordOrTilde = Character#isLetterOrDigit under JDK 21 = Unicode 15.0), ReadLineReader#readLine
(BufferedReader line split, one BOM, U+2013 -> '-'), ReadLineWithYamlHeader, Sub#fromFile, BlockUmlBuilder (!exit),
Pipe#readSingleDiagram, CommandPragma.
'welcome' = Java exit-0 Welcome page (counted as its own outcome; intended_valid=True).
'empty-description' = Java exit-200 error (intended_valid=False). 'unsure' predictions are marked in axes.
Usage: python3 -I gen_probes_cr4.py OUT_DIR"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
S = "@startuml\n"; E = "@enduml\n"
T = "Kea -> Tui : x %chr(92)\n"
ONE = "Kea -> Tui : one\n"; TWO = "Tui -> Kea : two\n"
SPR = "sprite $kiwi [4x4/16] {\n0000\n0F00\n00F0\n0000\n}\n"
def hdr(c): return f"{c} @startuml\n{c} Kea -> Tui : hdr\n{c} @enduml\n"
RES = {
 "cr4_empty.iuml": "",
 "cr4_blank.iuml": "\n",
 "cr4_subs.iuml": "!startsub EMPTY\n!endsub\n!startsub P\nKea -> Tui : p\n!endsub\n!startsub D\nKea –> Tui : dash\n!endsub\n",
 "puml-theme-cr4c.puml": "' only a comment\n",
 "puml-theme-cr4s.puml": "skinparam backgroundColor #EEEEEE\n",
 "cr4_w_two.iuml": "@startuml\nKea -> Tui : first\n@enduml\n@startuml\nKea -> Tui : second\n@enduml\n",
 "cr4_r_plain.iuml": "Kea -> Tui : plain\n",
 "cr4-a.iuml": "Kea -> Tui : dashpath\n",
 "cr4_r_cr.iuml": "Kea -> Tui : a\rTui -> Kea : b\r",
 "cr4_r_yaml.iuml": "---\ntitle: x\n---\nKea -> Tui : y\n",
 "cr4_w_yaml.iuml": "---\na: b\n---\n@startuml\nKea -> Tui : w\n@enduml\n",
 "cr4_ws_sub.iuml": "　!startsub P\nKea -> Tui : wsp\n!endsub\n",
 "cr4_r_pragma.iuml": "!pragma teoz true\n",
 "cr4_r_sprite.iuml": SPR,
 "cr4_r_dbom.iuml": "﻿﻿Kea -> Tui : dbom\n",
 "cr4_h_tje.iuml": hdr("Ᲊ"),      # Unicode 16.0 Lu; Cn in 15.0
 "cr4_h_a7cb.iuml": hdr("Ɤ"),     # Unicode 16.0 Lu; Cn in 15.0
 "cr4_h_a7ce.iuml": hdr("꟎"),     # Unicode 17.0 Lu; Cn in 15.0
 "cr4_h_088f.iuml": hdr("࢏"),     # Unicode 17.0 Lo; Cn in 15.0
 "cr4_h_thai.iuml": hdr("ก"),     # Lo in 15.0 (control)
 "cr4_h_digit.iuml": hdr("٣"),    # Nd in 15.0 (control)
}
C = []
def add(name, src, valid, axes, pred): C.append((name, src, valid, axes, pred))

# --- A: the TIM line count (UmlSource#getTotalLineCount == 2 -> Welcome; isEmpty -> Empty description) ---
add("cr4_cnt_pragma_bare", S + "!pragma\n" + E, False, ["count", "!pragma bare: PLAIN, count 3", "CommandPragma needs a NAME"], "syntax error (exit 200)")
add("cr4_cnt_pragma_bare_tail", S + "!pragma\n" + T + E, False, ["count", "!pragma bare + tail", "count 3"], "syntax error on !pragma (exit 200)")
add("cr4_cnt_pragma_upper_msg", S + "!PRAGMA teoz true\n" + ONE + E, False, ["count", "!PRAGMA: PLAIN; CommandPragma is case-sensitive", "unsure"], "syntax error (exit 200)")
add("cr4_cnt_pragma_continued_end", S + "!pragma teoz true \\\n" + E, True, ["count", "pragma ends with backslash", "joined with end: count 2"], "welcome")
add("cr4_cnt_pragma_from_include_tail", S + "!include cr4_r_pragma.iuml\n" + T + E, True, ["count", "!pragma from raw include", "count 3"], "teoz message 'x @enduml'")
add("cr4_cnt_pragma_from_variable_line", S + "!$p = \"!pragma teoz true\"\n$p\n" + ONE + E, True, ["count", "TIM-generated !pragma line", "unsure"], "teoz message 'one'")
add("cr4_cnt_option_unknown", S + "!option foo bar\n" + E, True, ["count", "!option consumed: count 2"], "welcome")
add("cr4_cnt_assume_alone", S + "!assume transparent light\n" + E, True, ["count", "!assume PLAIN: count 3", "unsure"], "an empty diagram (not Welcome)")
add("cr4_cnt_log_alone", S + "!log hello\n" + E, True, ["count", "!log consumed: count 2"], "welcome")
add("cr4_cnt_assert_alone", S + "!assert 1 == 1\n" + E, True, ["count", "!assert consumed: count 2"], "welcome")
add("cr4_cnt_undef_alone", S + "!undef NOPE\n" + E, True, ["count", "!undef consumed: count 2"], "welcome")
add("cr4_cnt_include_empty", S + "!include cr4_empty.iuml\n" + E, True, ["count", "include of an empty file: count 2"], "welcome")
add("cr4_cnt_include_blank_line", S + "!include cr4_blank.iuml\n" + E, False, ["count", "include of one blank line: count 3", "UmlSource#isEmpty"], "empty-description (exit 200)")
add("cr4_cnt_includesub_empty", S + "!includesub cr4_subs.iuml!EMPTY\n" + E, True, ["count", "empty sub: count 2"], "welcome")
add("cr4_cnt_theme_local_comment_only", S + "!theme cr4c from .\n" + E, True, ["count", "local theme with no TIM lines: count 2"], "welcome")
add("cr4_cnt_theme_local_one_skinparam", S + "!theme cr4s from .\n" + E, True, ["count", "local theme with one line: count 3", "unsure"], "an empty diagram (not Welcome)")
add("cr4_cnt_theme_unshipped_alone", S + "!theme mono\n" + E, True, ["count", "unshipped bundled theme alone", "unsure"], "an empty diagram (not Welcome)")
add("cr4_cnt_exit_alone", S + "!exit\n" + E, True, ["count", "BlockUmlBuilder !exit pauses; end line appended", "count 2"], "welcome")
add("cr4_exit_mid", S + ONE + "!exit\n" + TWO + E, True, ["BlockUmlBuilder !exit", "lines after !exit dropped"], "'one' only")
add("cr4_cnt_blank_three", S + "\n" + E, False, ["count", "one blank line: count 3", "UmlSource#isEmpty"], "empty-description (exit 200)")
add("cr4_cnt_ws_only_line", S + " \t \n" + E, False, ["count", "whitespace-only line: count 3", "UmlSource#isEmpty (trin)"], "empty-description (exit 200)")
add("cr4_cnt_empty_var_line", S + "!$e = \"\"\n$e\n" + E, False, ["count", "TIM line evaluating to empty", "UmlSource#isEmpty"], "empty-description (exit 200)")
add("cr4_cnt_tim_comment_line", S + "!$c = \"' hidden\"\n$c\n" + E, False, ["count", "TIM line evaluating to a comment", "UmlSource#isEmpty COMMENT_LINE"], "empty-description (exit 200)")
add("cr4_cnt_blank_then_message", S + "\n" + ONE + E, True, ["count", "control: blank then content"], "'one'")

# --- B: one-line sources and end lines swallowed by a definition, in several families ---
add("cr4_one_mindmap_procdef", "@startmindmap\n!procedure $p()\n* a\n@endmindmap\n!endprocedure\n@endmindmap\n", False, ["one-line", "mindmap", "PSystemCommandFactory isEmpty"], "empty-description (exit 200)")
add("cr4_one_gantt_procdef", "@startgantt\n!procedure $p()\n[T] lasts 1 day\n@endgantt\n!endprocedure\n@endgantt\n", False, ["one-line", "gantt"], "empty-description (exit 200)")
add("cr4_one_wbs_function", "@startwbs\n!function $f()\n!return 1\n\\endwbs\n!endfunction\n@endwbs\n", False, ["one-line", "wbs", "\\end form"], "empty-description (exit 200)")
add("cr4_one_salt_definelong", "@startsalt\n!definelong D()\n{ x }\n@endsalt\n!enddefinelong\n@endsalt\n", False, ["one-line", "salt", "unsure"], "empty-description (exit 200)")
add("cr4_one_json_procdef", "@startjson\n!procedure $p()\n{}\n@endjson\n!endprocedure\n@endjson\n", False, ["one-line", "json (not a command factory)", "unsure"], "an error (exit 200)")
add("cr4_two_mindmap_content_then_procdef", "@startmindmap\n* root\n!procedure $p()\n@endmindmap\n!endprocedure\n@endmindmap\n", True, ["two-line", "mindmap: no Welcome factory"], "mindmap 'root'")
add("cr4_two_class_content_then_function_end", S + "class Kea\n!function $f()\n@enduml\n!endfunction\n" + E, True, ["two-line", "class content", "end swallowed by !function"], "welcome")
add("cr4_two_blank_then_procdef_end", S + "\n!procedure $p()\n@enduml\n!endprocedure\n" + E, True, ["two-line", "blank line then swallowed end: count 2"], "welcome")

# --- C: the whitespace set at directive checks (TLineType [%s]*: ASCII \s + NBSP; isExit/isEndDirective: isWhitespace) ---
add("cr4_ws_ideo_before_assign", S + "　!$x = \"a\"\nKea -> Tui : $x\n" + E, False, ["whitespace", "U+3000 before a TIM assignment: PLAIN"], "syntax error (exit 200)")
add("cr4_ws_ideo_before_comment", S + "　' c\n" + ONE + E, False, ["whitespace", "U+3000 before ': PLAIN, not a comment"], "syntax error (exit 200)")
add("cr4_ws_ideo_before_procedure", S + "　!procedure $p()\nKea -> Tui : p\n!endprocedure\n$p()\n" + E, False, ["whitespace", "U+3000 before !procedure: PLAIN"], "syntax error (exit 200)")
add("cr4_ws_ideo_before_if", S + "　!if 1\n" + ONE + "!endif\n" + E, False, ["whitespace", "U+3000 before !if: PLAIN; stray !endif"], "error (exit 200)")
add("cr4_ws_nbsp_before_assign", S + " !$x = \"a\"\nKea -> Tui : $x\n" + E, True, ["whitespace", "NBSP before a TIM assignment: [%s] includes U+00A0"], "message 'a'")
add("cr4_ws_vt_before_define", S + "\x0b!define WHO Moa\nKea -> WHO : a\n" + E, True, ["whitespace", "VT before !define: ASCII \\s"], "Kea -> Moa")
add("cr4_ws_us_before_assign", S + "\x1f!$x = \"a\"\nKea -> Tui : $x\n" + E, False, ["whitespace", "U+001F before an assignment: not \\s"], "syntax error (exit 200)")
add("cr4_ws_ideo_startsub_in_includesub", S + "!includesub cr4_ws_sub.iuml!P\n" + E, False, ["whitespace", "Sub#fromFile: U+3000 before !startsub is no marker", "cannot include"], "error (exit 200)")
add("cr4_ws_ideo_exit", S + ONE + "　!exit　\n" + TWO + E, True, ["whitespace", "StartUtils#isExit trims Character#isWhitespace"], "'one' only")
add("cr4_ws_ideo_before_at_end", S + ONE + "　@enduml\n" + TWO + E, True, ["whitespace", "U+3000 before @enduml ends the block"], "'one' only")

# --- D: ReadLineReader (BOM, U+2013, line split) in the main source and includes; ReadLineWithYamlHeader ---
add("cr4_main_bom_line2", S + "﻿Kea -> Tui : bom\n" + E, True, ["BOM", "main source body line"], "message 'bom'")
add("cr4_main_bom_before_directive", S + "﻿!$x = \"a\"\nKea -> Tui : $x\n" + E, True, ["BOM", "main source, before a directive"], "message 'a'")
add("cr4_main_endash_label", S + "Kea -> Tui : a – b\n" + E, True, ["U+2013", "main source label"], "message 'a - b'")
add("cr4_main_endash_arrow", S + "Kea –> Tui : dash\n" + E, True, ["U+2013", "main source arrow"], "Kea -> Tui 'dash'")
add("cr4_main_endash_include_path", S + "!include cr4–a.iuml\n" + E, True, ["U+2013", "main source !include path becomes cr4-a.iuml"], "message 'dashpath'")
add("cr4_inc_endash_includesub_body", S + "!includesub cr4_subs.iuml!D\n" + E, True, ["U+2013", "!includesub body"], "Kea -> Tui 'dash'")
add("cr4_inc_double_bom", S + "!include cr4_r_dbom.iuml\n" + E, False, ["BOM", "two BOMs: one stays", "unsure"], "syntax error (BOM before Kea)")
add("cr4_inc_cr_only_lines", S + "!include cr4_r_cr.iuml\n" + E, True, ["line split", "BufferedReader splits on lone CR"], "'a' and 'b'")
add("cr4_main_cr_only_lines", "@startuml\rKea -> Tui : a\r@enduml\r", True, ["line split", "Pipe BufferedReader splits on lone CR"], "message 'a'")
add("cr4_inc_yaml_header_raw", S + "!include cr4_r_yaml.iuml\n" + E, True, ["ReadLineWithYamlHeader", "raw include"], "message 'y' only")
add("cr4_inc_yaml_header_wrapped", S + "!include cr4_w_yaml.iuml\n" + E, True, ["ReadLineWithYamlHeader", "wrapped include (extraction first)"], "message 'w'")

# --- E: the !N / !id suffix (executeInclude lastIndexOf; includesub indexOf) ---
add("cr4_sfx_bang_empty", S + "!include cr4_w_two.iuml!\n" + E, True, ["suffix", "empty suffix"], "'first'")
add("cr4_sfx_bang0", S + "!include cr4_w_two.iuml!0\n" + E, True, ["suffix", "!0"], "'first'")
add("cr4_sfx_two_bangs", S + "!include cr4_w_two.iuml!1!2\n" + E, True, ["suffix", "lastIndexOf leaves cr4_w_two.iuml!1: no such file", "unsure"], "nothing included: welcome")
add("cr4_sfx_include_many", S + "!include_many cr4_r_plain.iuml!1\n" + E, True, ["suffix", "!include_many"], "message 'plain'")
add("cr4_sfx_include_once_twice", S + "!include_once cr4_r_plain.iuml!1\n!include_once cr4_r_plain.iuml!2\n" + E, True, ["suffix", "!include_once twice: filesUsedCurrent is never filled"], "'plain' twice")
add("cr4_sfx_includesub_double_bang", S + "!includesub cr4_subs.iuml!P!x\n" + E, False, ["suffix", "includesub indexOf: block 'P!x' missing"], "cannot include (exit 200)")
add("cr4_sfx_quoted_plain", S + "!include \"cr4_r_plain.iuml\"\n" + E, True, ["suffix", "quoted path control", "unsure"], "message 'plain'")
add("cr4_sfx_quoted_bang", S + "!include \"cr4_w_two.iuml!1\"\n" + E, True, ["suffix", "quoted path with suffix", "unsure"], "'first'")

# --- F: Character#isLetterOrDigit under Unicode 15.0 in the DiagramDetector header test ---
add("cr4_hdr_u16_cyrillic_tje", S + "!include cr4_h_tje.iuml\n" + E, True, ["header", "U+1C89 (Unicode 16 letter, unassigned in 15.0)"], "wrapped: message 'hdr'")
add("cr4_hdr_u16_latin_a7cb", S + "!include cr4_h_a7cb.iuml\n" + E, True, ["header", "U+A7CB (Unicode 16 letter)"], "wrapped: message 'hdr'")
add("cr4_hdr_u17_latin_a7ce", S + "!include cr4_h_a7ce.iuml\n" + E, True, ["header", "U+A7CE (Unicode 17 letter)"], "wrapped: message 'hdr'")
add("cr4_hdr_u17_arabic_088f", S + "!include cr4_h_088f.iuml\n" + E, True, ["header", "U+088F (Unicode 17 letter)"], "wrapped: message 'hdr'")
add("cr4_hdr_u15_thai_control", S + "!include cr4_h_thai.iuml\n" + E, False, ["header", "U+0E01 is a 15.0 letter: raw include"], "syntax error (exit 200)")
add("cr4_hdr_u15_digit_control", S + "!include cr4_h_digit.iuml\n" + E, False, ["header", "U+0663 is Nd: raw include"], "syntax error (exit 200)")

# --- G: sprites as diagram commands after TIM (parity-recovery's post-TIM extraction dropped) ---
add("cr4_sprite_from_procedure", S + "!procedure $s()\n" + SPR + "!endprocedure\n$s()\nKea -> Tui : <$kiwi>\n" + E, True, ["sprite", "declared by a procedure"], "message with sprite")
add("cr4_sprite_from_include", S + "!include cr4_r_sprite.iuml\nKea -> Tui : <$kiwi>\n" + E, True, ["sprite", "declared in a raw include"], "message with sprite")
add("cr4_sprite_continued_header", S + "sprite $kiwi \\\n[4x4/16] {\n0000\n0F00\n00F0\n0000\n}\nKea -> Tui : <$kiwi>\n" + E, True, ["sprite", "continued header line"], "message with sprite")

# --- H: end lines swallowed by other CodeIterators (T14.66 extension candidates) ---
add("cr4_end_in_startsub_unterminated", S + ONE + "!startsub X\n" + E, True, ["swallowed end", "CodeIteratorSub without !endsub", "unsure"], "welcome")
add("cr4_end_in_foreach_unterminated", S + ONE + "!foreach $i in [1]\n" + E, False, ["swallowed end", "!foreach without !endfor", "unsure"], "error (exit 200)")
add("cr4_end_in_while_unterminated", S + ONE + "!$i = 0\n!while $i < 1\n" + E, False, ["swallowed end", "!while without !endwhile", "unsure"], "error (exit 200)")
add("cr4_end_in_if_unterminated_active", S + ONE + "!if 1\n" + E, False, ["swallowed end", "active !if without !endif", "unsure"], "error (exit 200)")

cases = []
for name, src, valid, axes, pred in C:
    data = src.encode("utf-8")
    (out / f"{name}.puml").write_bytes(data)
    cases.append(dict(name=name, intended_valid=valid, axes=axes, java_prediction=pred, crlf="\r\n" in src))
for p, t in RES.items(): (out / p).write_bytes(t.encode("utf-8"))
assert len({c["name"] for c in cases}) == len(cases)
(out / "cases.json").write_text(json.dumps(cases, indent=1, ensure_ascii=False))
(out / "resources.json").write_text(json.dumps(sorted(RES), indent=1))
print(len(cases), sum(c["intended_valid"] for c in cases), len(RES))
