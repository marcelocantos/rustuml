"""Checker round-7 (Opus) held-out probes, batch Q8, for T14.22 round 7 (maker tip df078f7d7, code 16b1c4dcf).
Written before any Java result for these inputs exists. Java predictions come from the pinned source (71806a237):
  - Pipe#readSingleDiagram (Pipe.java:180-225), read by a BufferedReader (\n, \r and \r\n all end a line):
    a line startsWith("@@@format ") goes to manageFormat (which may switch the output format) and is never appended;
    while NO_CONTENT a line whose String#trim is empty is not appended; a column-0 "@start" is the mark and the text
    before it is discarded; with no mark the text is "@startuml\n" + sb + "@enduml\n". data-source-line is the 0-based
    position in Pipe's text.
  - BlockUmlBuilder (91-135): UncommentReadLine(ReadLineReader) through Preprocessor (ReadFilterMergeLines); every
    isStartDirective line restarts the block; isEndDirective (case-sensitive "end") closes it.
  - ReadLineReader#readLine (89-103): br.readLine(), one leading U+FEFF dropped, U+2013 -> '-'. Nothing else
    (the U+00A0/quote replacements are commented out).
  - DiagramType#findStartTypes/getTypes/check (69-220): 'start' and each key with regionMatches(true); the type switch is
    Character.toLowerCase of the first unit. Non-ASCII units folding to an ASCII key letter: U+0130, U+0131 (i),
    U+017F (s), U+212A (k) (support/fold_check.py over UCD 15.0.0 and 13.0.0).
  - PSystemWelcomeFactory: a source of exactly 2 lines after TIM is the Welcome page.
intended_valid is the checker's Java-validity prediction; 'unsure' is marked in axes; 'java_lines' gives the
predicted data-source-line of the messages in order.
Usage: python3 -I gen_probes_q8.py OUT_DIR"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
B = "\\"
RES = {
 "q8_fmtline.iuml": "Quern -> Vane : inc\n@@@format svg\n",
 "q8_parts.iuml": "participant Quern\nparticipant Vane\n",
 "q8-dash.iuml": "participant Vane\nparticipant Quern\n",
}
C = []
def case(name, text, valid, axes, crlf=False):
    C.append(dict(name=name, text=text, intended_valid=valid, axes=axes, crlf=crlf))
# --- A. @@@format (Pipe.java:188-189) ---
case("q8_fmt_mid_marked_seq", "@startuml\nQuern -> Vane : one\n@@@format svg\nVane -> Quern : two\n@enduml\n", True, ["fmt", "marked", "mid", "java_lines 1,2"])
case("q8_fmt_lead_marked_seq_two", "@@@format svg\n@startuml\nQuern -> Vane : one\nVane -> Quern : two\n@enduml\n", True, ["fmt", "marked", "before mark", "java_lines 1,2"])
case("q8_fmt_mid_bs_seq_svg", f"{B}startuml\nQuern -> Vane : one\n@@@format svg\nVane -> Quern : two\n{B}enduml\n", True, ["fmt", "no mark", "mid", "java_lines 2,3"])
case("q8_fmt_in_continuation_bs", f"{B}startuml\nQuern -> Vane : one {B}\n@@@format svg\ntail\nVane -> Quern : two\n{B}enduml\n", True, ["fmt", "no mark", "continuation joins across the dropped line", "java_lines 2,4"])
case("q8_fmt_tab_marked_invalid", "@startuml\nQuern -> Vane : one\n@@@format\tsvg\n@enduml\n", False, ["fmt", "tab is not the separator: line reaches the parser"])
case("q8_fmt_space_only_marked", "@startuml\nQuern -> Vane : one\n@@@format \nVane -> Quern : two\n@enduml\n", True, ["fmt", "marked", "prefix only", "java_lines 1,2"])
case("q8_fmt_upper_invalid", "@startuml\nQuern -> Vane : one\n@@@FORMAT svg\n@enduml\n", False, ["fmt", "case-sensitive prefix"])
case("q8_fmt_crlf_mid_bs", f"{B}startuml\nQuern -> Vane : one\n@@@format svg\nVane -> Quern : two\n{B}enduml\n", True, ["fmt", "CRLF", "java_lines 2,3"], crlf=True)
case("q8_fmt_in_include_invalid", f"{B}startuml\n!include q8_fmtline.iuml\nVane -> Quern : two\n{B}enduml\n", False, ["fmt", "include is not read by Pipe"])
case("q8_fmt_two_mid_marked", "@startuml\nQuern -> Vane : one\n@@@format svg\n@@@format foo\nVane -> Quern : two\n@enduml\n", True, ["fmt", "marked", "two dropped", "java_lines 1,2"])
case("q8_fmt_after_blank_lead_bs", f"\n\t\n@@@format svg\n{B}startuml\nQuern -> Vane : one\n{B}enduml\n", True, ["fmt", "blank lead", "java_lines 2"])
case("q8_fmt_bom_prefixed_invalid", "@startuml\nQuern -> Vane : one\n﻿@@@format svg\n@enduml\n", False, ["fmt", "BOM hides the prefix from Pipe; ReadLineReader then strips it"])
case("q8_fmt_mid_error_line_bs", f"{B}startuml\nQuern -> Vane : one\n@@@format svg\nnot a command here\n{B}enduml\n", False, ["fmt", "error line 3 (Pipe text)"])
case("q8_fmt_include_parts_bs", f"{B}startuml\n!include q8_parts.iuml\n@@@format svg\nQuern -> Vane : one\n{B}enduml\n", True, ["fmt", "include", "java_lines 3"])
case("q8_inc_parts_bs_control", f"{B}startuml\n!include q8_parts.iuml\nQuern -> Vane : one\n{B}enduml\n", True, ["control for q8_fmt_include_parts_bs", "java_lines 3"])
# --- B. leading trim-empty lines, blank lines, line ends ---
case("q8_lead_ws_nostart_seq", "\n \n\tQuern -> Vane : one\nVane -> Quern : two\n", True, ["lead", "no start at all", "java_lines 1,2"])
case("q8_lead_blank_comment_only", "\n\n' just a remark\n", False, ["lead", "Welcome (2 lines after TIM)", "unsure"])
case("q8_lead_crlf_mixed_bs", f"\r\n \r\n{B}startuml\nQuern -> Vane : one\r\n{B}enduml\n", True, ["lead", "mixed CRLF/LF", "java_lines 2"])
case("q8_lone_cr_bs", f"{B}startuml\rQuern -> Vane : one\r{B}enduml\r", True, ["line ends", "lone CR ends a line (BufferedReader)", "java_lines 2"])
case("q8_cr_inside_marked", "@startuml\nQuern -> Vane : one\rVane -> Quern : two\n@enduml\n", True, ["line ends", "lone CR inside", "java_lines 1,2"])
case("q8_blank_after_start_bs", f"{B}startuml\n\n\nQuern -> Vane : one\n\n{B}enduml\n", True, ["blank after content is appended", "java_lines 4"])
case("q8_lead_blank_error_bs", f"\n\n{B}startuml\nQuern -> Vane : one\nnot a command here\n{B}enduml\n", False, ["lead", "error line 3 (Pipe text)"])
case("q8_welcome_lead_ws_bs", f"\t\n \n{B}startuml\n' Quern note\n{B}enduml\n", False, ["lead", "comment-only body: Welcome", "unsure"])
case("q8_welcome_lead_ctrl_bs", f"\u001f\n{B}startuml\n{B}enduml\n", False, ["lead", "U+001F trim-empty", "Welcome", "unsure"])
case("q8_noc_ctrl_fmt_only", "\n\u0001\n@@@format svg\n\t\n", False, ["NO_CONTENT: Pipe returns null, no diagram", "unsure exit"])
case("q8_lead_blank_cont_nostart_class", "\n\t\nclass Quern \\\n<<tag>>\nclass Vane\nQuern --> Vane\n", True, ["lead", "no start", "continuation", "class"])
case("q8_lead_blank_assert_bs", f"\n\n{B}startuml\n!assert 1 == 2 : boom\nQuern -> Vane : one\n{B}enduml\n", False, ["lead", "TIM error"])
# --- C. regionMatches(true) case folding ---
case("q8_cf_ws_at_longs_upper", "  @ſTARTUML\nQuern -> Vane : one\n@enduml\n", True, ["fold", "U+017F in start", "indented: no Pipe mark", "java_lines 2"])
case("q8_cf_bs_longs_upper_mindmap", f"{B}ſTARTMINDMAP\n* Quern\n** Vane\n{B}endmindmap\n", True, ["fold", "U+017F in start", "upper type"])
case("q8_cf_upper_dotted_mindmap", f"{B}startMİNDMAP\n* Quern\n** Vane\n{B}endmindmap\n", True, ["fold", "U+0130 in type"])
case("q8_cf_combining_dot_invalid", f"{B}startmİndmap\n* Quern\n** Vane\n{B}endmindmap\n", False, ["fold", "I + U+0307 is not one unit: UNKNOWN"])
case("q8_cf_fullwidth_invalid", f"{B}startｕml\nQuern -> Vane : one\n{B}enduml\n", False, ["fold", "fullwidth u: UNKNOWN"])
case("q8_cf_upper_at_col0_restart", "@STARTUML\nQuern -> Vane : one\n@enduml\n", True, ["fold", "upper start is no Pipe mark", "java_lines 2"])
case("q8_cf_upper_end_invalid", "@STARTUML\nQuern -> Vane : one\n@ENDUML\n", False, ["fold", "end is case-sensitive: @ENDUML reaches the parser", "unsure"])
case("q8_cf_sharp_s_start_invalid", f"{B}ßtartuml\nQuern -> Vane : one\n{B}enduml\n", False, ["fold", "U+00DF does not fold to s: no start directive"])
# --- D. ReadLineReader ---
case("q8_rlr_bom_marked_first", "﻿@startuml\nQuern -> Vane : one\n@enduml\n", True, ["rlr", "BOM hides the mark from Pipe", "java_lines 2"])
case("q8_rlr_bom_continuation_bs", f"{B}startuml\nQuern -> Vane : one {B}\n﻿tail\n{B}enduml\n", True, ["rlr", "BOM on a continued line", "java_lines 2"])
case("q8_rlr_bom_comment_marked", "@startuml\n﻿' remark\nQuern -> Vane : one\n@enduml\n", True, ["rlr", "BOM before a comment", "java_lines 2"])
case("q8_rlr_bom_tim_var_marked", "@startuml\n﻿!$w = \"Vane\"\nQuern -> $w : one\n@enduml\n", True, ["rlr", "BOM before a TIM line", "java_lines 2"])
case("q8_rlr_endash_class_arrow", "@startuml\nclass Quern\nclass Vane\nQuern ––> Vane\n@enduml\n", True, ["rlr", "U+2013 arrow"])
case("q8_rlr_emdash_invalid", "@startuml\nQuern —> Vane : one\n@enduml\n", False, ["rlr", "U+2014 is not replaced", "unsure"])
case("q8_rlr_minus_sign_invalid", "@startuml\nQuern −> Vane : one\n@enduml\n", False, ["rlr", "U+2212 is not replaced", "unsure"])
case("q8_rlr_figure_dash_invalid", "@startuml\nQuern ‒> Vane : one\n@enduml\n", False, ["rlr", "U+2012 is not replaced", "unsure"])
case("q8_rlr_endash_header_mixed_bs", f"– {B}startuml\n- Quern -> Vane : one\n- {B}enduml\n", True, ["rlr", "U+2013 header equals '- ' after the reader", "java_lines 2"])
case("q8_rlr_smart_quotes_label", "@startuml\nQuern -> Vane : “hi” ‘yo’\n@enduml\n", True, ["rlr", "quotes are not replaced"])
case("q8_rlr_endash_include_path", "@startuml\n!include q8–dash.iuml\nQuern -> Vane : one\n@enduml\n", True, ["rlr", "U+2013 in an include path becomes '-'"])
case("q8_rlr_nbsp_kept_label", "@startuml\nQuern -> Vane : a b\n@enduml\n", True, ["rlr", "U+00A0 is not replaced"])
case("q8_rlr_bom_mid_line_kept", "@startuml\nQuern -> Vane : a﻿b\n@enduml\n", True, ["rlr", "inner U+FEFF kept"])
# --- E. combinations ---
case("q8_mix_crlf_fmt_bom_cont", f"{B}startuml\n﻿Quern -> Vane : one {B}\n@@@format svg\ntail\nVane -> Quern : two\n{B}enduml\n", True, ["fmt", "rlr", "continuation", "CRLF", "java_lines 2,4"], crlf=True)
case("q8_mix_lead_bom_blank_ws", f"﻿ \n{B}startuml\nQuern -> Vane : one\n{B}enduml\n", True, ["rlr", "BOM+space line is content for String#trim", "java_lines 3"])
case("q8_tim_def_after_blank_lead_bs", f"\n\n{B}startuml\n!$w = \"Vane\"\nQuern -> $w : one\n{B}enduml\n", True, ["lead", "TIM", "java_lines 3"])
for c in C:
    t = c["text"].replace("\r\n", "\n").replace("\n", "\r\n") if c["crlf"] else c["text"]
    (out / f"{c['name']}.puml").write_bytes(t.encode("utf-8"))
for p, t in RES.items(): (out / p).write_bytes(t.encode("utf-8"))
(out / "cases.json").write_text(json.dumps([{k: v for k, v in c.items() if k != "text"} for c in C], indent=1, ensure_ascii=False))
(out / "resources.json").write_text(json.dumps(sorted(RES)))
print(len(C), "cases,", sum(c["intended_valid"] for c in C), "predicted valid,", len(RES), "resources")
