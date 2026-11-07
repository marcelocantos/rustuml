"""Maker round-3 held-out probes (batch MR3) for T14.22 round 3: the out-of-band stand-in, Java's two-line
source rule after the post-TIM join, StartUtils#isEndDirective for '\\end', and DiagramDetector's wrapped includes.
Written before any Java result for these inputs exists. intended_valid is the maker's Java prediction from the
pinned source trace (account.json); a Welcome page is predicted as not valid. Validity is decided later from
raw Java SVG + stderr only.
Usage: python3 -I gen_probes.py OUT_DIR"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
RES = {
    "mr3_tail.iuml": "Kea -> Tui : tail \\\n",
    "mr3_bad_tail.iuml": "not a command at all \\\n",
    "mr3_link_tail.iuml": "Ash --> Elm : feeds \\\n",
    "mr3_rs.iuml": "Kea -> Tui : inc\n\x1eRSP\n",
    "mr3_wrap_preamble.iuml": "!define WHO Moa\n@startuml\nKea -> WHO : w\n@enduml\n",
    "mr3_wrap_trailer.iuml": "@startuml\nKea -> Tui : inside\n@enduml\nTui -> Kea : outside\n",
    "mr3_wrap_continued.iuml": "@startuml\nKea -> Tui : first \\\nsecond\n@enduml\n",
    "mr3_wrap_indented.iuml": "  @startuml\n  Kea -> Tui : indented\n  @enduml\n",
    "mr3_wrap_comment_header.iuml": "' @startuml\n'Kea -> Tui : commented\n' @enduml\n",
    "mr3_wrap_two_blocks.iuml": "@startuml\nKea -> Tui : block one\n@enduml\n@startuml\nTui -> Kea : block two\n@enduml\n",
    "mr3_wrap_class.iuml": "@startuml\nclass Rimu \\\n<<native>>\n@enduml\n",
    "mr3_raw_class.iuml": "class Rimu \\\n",
}
S = "@startuml\n"; E = "@enduml\n"
# (name, source, intended_valid (Java prediction), axes, java_prediction)
C = [
 # --- A: stand-in collision (obligation 1) ---
 ("mr3_seq_chr30_rsp_after_directive", S + "!$x = 1\nKea -> Tui : one\n%chr(30)RSP\n" + E, False,
  ["stand-in", "%chr(30)RSP after a directive"], "no command accepts the line: syntax error (exit 200)"),
 ("mr3_seq_procedure_emits_rs_line", S + "!procedure $p()\n%chr(30)RSP\n!endprocedure\nKea -> Tui : one\n$p()\n" + E, False,
  ["stand-in", "procedure output"], "syntax error on the emitted line"),
 ("mr3_seq_include_rs_line", S + "!include mr3_rs.iuml\n" + E, False,
  ["stand-in", "raw U+001E RSP in an included file"], "syntax error on the included line"),
 ("mr3_seq_rs_line_in_if", S + "!if 1\n%chr(30)RSP\n!endif\nKea -> Tui : one\n" + E, False,
  ["stand-in", "inside an active !if"], "syntax error"),
 # --- B: two-line source rule (obligation 2) ---
 ("mr3_seq_end_continued_one_line", S + "Kea -> Tui : one\n@enduml \\\n", False,
  ["two-line", "end line ends with backslash, one line before"], "the pending end is dropped: 2 lines -> Welcome"),
 ("mr3_seq_end_continued_two_lines", S + "Kea -> Tui : one\nTui -> Kea : two\n@enduml \\\n", True,
  ["two-line control", "end line ends with backslash, two lines before"], "3 lines: messages 'one' and 'two'"),
 ("mr3_seq_bad_tail_then_end", S + "!include mr3_bad_tail.iuml\n" + E, False,
  ["two-line", "joined line no command accepts"], "PSystemWelcomeFactory runs first: Welcome, not a syntax error"),
 ("mr3_class_link_tail_then_end", S + "class Ash\nclass Elm\n!include mr3_link_tail.iuml\n" + E, True,
  ["two-line control", "class", "joined link before end with two classes"], "4 lines: class diagram with link 'feeds @enduml'"),
 ("mr3_class_only_link_tail_then_end", S + "!include mr3_link_tail.iuml\n" + E, False,
  ["two-line", "class link tail absorbs the end"], "2 lines -> Welcome (Welcome factory precedes class)"),
 ("mr3_seq_chr92_tail_then_tab_end", S + "Kea -> Tui : t %chr(92)\n\t@enduml\n", False,
  ["two-line", "tab-indented end is BlockUmlBuilder's end"], "'t \\t@enduml' joined: 2 lines -> Welcome"),
 ("mr3_seq_tail_then_end_three_lines", S + "Tui -> Kea : one\n!include mr3_tail.iuml\n" + E, True,
  ["two-line control", "three lines"], "messages 'one' and 'tail @enduml'"),
 ("mr3_seq_tail_then_backslash_end_then_text", S + "!include mr3_tail.iuml\n\\enduml\nTui -> Kea : after\n", False,
  ["two-line", "\\enduml ends the block", "text after it"], "'tail \\enduml': 2 lines -> Welcome"),
 ("mr3_seq_tail_then_option_then_msg", S + "!include mr3_tail.iuml\n!option fake value\nTui -> Kea : two\n" + E, True,
  ["stand-in", "!option is a TIM directive"], "'tail Tui -> Kea : two' (the !option line is no TIM line)"),
 # --- C: '\\end' is an end directive (BlockUmlBuilder) ---
 ("mr3_seq_backslash_end_plain", S + "Kea -> Tui : one\n\\enduml\n", True,
  ["end-directive", "\\enduml without a continuation"], "block ends at \\enduml: message 'one'"),
 ("mr3_seq_backslash_end_then_more", S + "Kea -> Tui : one\n\\enduml\nTui -> Kea : two\n" + E, True,
  ["end-directive", "content after \\enduml"], "message 'one' only"),
 # --- D: wrapped includes (obligation 3) ---
 ("mr3_seq_wrapped_preamble_dropped", S + "!include mr3_wrap_preamble.iuml\n" + E, True,
  ["wrapped-include", "lines before the start are not read"], "WHO is not defined: participant 'WHO', message 'w'"),
 ("mr3_seq_wrapped_trailer_dropped", S + "!include mr3_wrap_trailer.iuml\n" + E, True,
  ["wrapped-include", "lines after the end are not read"], "message 'inside' only"),
 ("mr3_seq_wrapped_continued_line", S + "!include mr3_wrap_continued.iuml\nTui -> Kea : main\n" + E, True,
  ["wrapped-include", "continuation merged inside the file"], "messages 'first second' and 'main'"),
 ("mr3_seq_wrapped_indented", S + "!include mr3_wrap_indented.iuml\n" + E, True,
  ["wrapped-include", "indented start and end"], "message 'indented'"),
 ("mr3_seq_wrapped_comment_header", S + "!include mr3_wrap_comment_header.iuml\n" + E, True,
  ["wrapped-include", "UncommentReadLine header removal"], "message 'commented'"),
 ("mr3_seq_wrapped_two_blocks", S + "!include mr3_wrap_two_blocks.iuml\n" + E, True,
  ["wrapped-include", "first block only"], "message 'block one' only"),
 ("mr3_class_wrapped_continued", S + "!include mr3_wrap_class.iuml\n" + E, True,
  ["wrapped-include", "class", "continuation inside the file"], "class Rimu with stereotype native"),
 ("mr3_class_raw_tail_control", S + "!include mr3_raw_class.iuml\n<<native>>\nclass Kauri\n" + E, True,
  ["control", "unwrapped include tail joins after TIM"], "'class Rimu <<native>>' and class Kauri"),
]
for name, src, valid, axes, pred in C:
    (out / f"{name}.puml").write_bytes(src.encode())
for path, text in RES.items():
    (out / path).write_bytes(text.encode())
cases = [dict(name=n, intended_valid=v, axes=a, java_prediction=p, crlf=False) for n, _, v, a, p in C]
(out / "cases.json").write_text(json.dumps(cases, indent=1, ensure_ascii=False))
(out / "resources.json").write_text(json.dumps(sorted(RES), indent=1))
print(len(cases), sum(c["intended_valid"] for c in cases), len(RES))
