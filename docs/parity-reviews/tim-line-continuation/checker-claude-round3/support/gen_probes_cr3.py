"""Checker round-3 held-out probes (batch CR3) for T14.22 round 3 (maker tip f2accc662, code d28dd64a9).
Written before any Java result for these inputs exists. intended_valid is the checker's Java prediction from the
pinned source trace (BlockUmlBuilder, ReadFilterMergeLines, TLineType/TContext, UmlSource#loadInternal,
PSystemBuilder/PSystemWelcomeFactory, PSystemCommandFactory, TContext#executeInclude, DiagramDetector,
DiagramExtractor, UncommentReadLine, ReadLineReader). Validity is decided later from raw Java SVG + stderr only.
'welcome' predictions are a distinct outcome (Java exit 0 Welcome page), recorded as intended_valid=True.
Usage: python3 -I gen_probes_cr3.py OUT_DIR"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
S = "@startuml\n"; E = "@enduml\n"
T = "Kea -> Tui : x %chr(92)\n"          # a post-TIM tail (TIM output ends with a backslash)
Y = "Tui -> Kea : y\n"
ONE = "Kea -> Tui : one\n"; TWO = "Tui -> Kea : two\n"
RES = {
 "cr3_directives.iuml": "!$inc = 1\n' only directives and comments\n",
 "cr3_w_three.iuml": "@startuml\nKea -> Tui : first\n@enduml\n@startuml\nKea -> Tui : second\n@enduml\n@startuml\nKea -> Tui : third\n@enduml\n",
 "cr3_w_ids.iuml": "@startuml(id=alpha)\nKea -> Tui : alpha\n@enduml\n@startuml(id=beta)\nKea -> Tui : beta\n@enduml\n",
 "cr3_w_mixed.iuml": "@startmindmap\n* mm\n@endmindmap\n@startuml\nKea -> Tui : uml\n@enduml\n",
 "cr3_w_hash.iuml": "## @startuml\n##Kea -> Tui : hashed\n#\n##@enduml\n##Kea -> Tui : not this\n",
 "cr3_w_angle.iuml": "<x> @startuml\n<x>Kea -> Tui : angled\n<x> @enduml\n",
 "cr3_w_sup.iuml": "\u00b2 @startuml\n\u00b2 Kea -> Tui : sup\n\u00b2 @enduml\n",
 "cr3_w_bom.iuml": "\ufeff@startuml\nKea -> Tui : bom\n@enduml\ntrailer junk\n",
 "cr3_w_bom_hash.iuml": "\ufeff# @startuml\n# Kea -> Tui : bomhash\n# @enduml\n",
 "cr3_w_bom_lines.iuml": "@startuml\n\ufeffKea -> Tui : midbom\n@enduml\n",
 "cr3_r_bom.iuml": "\ufeffKea -> Tui : rawbom\n",
 "cr3_w_crlf_cont.iuml": "@startuml\r\nKea -> Tui : c1 \\\r\n' skip\r\n/' skip2 '/\r\nc2\r\n@enduml\r\n",
 "cr3_w_tail_second.iuml": "@startuml\nKea -> Tui : x \\\n@enduml\n@startuml\nTui -> Kea : y\n@enduml\n",
 "cr3_w_outer.iuml": "@startuml\n!include cr3_w_inner.iuml\nTui -> Kea : outer\n@enduml\n",
 "cr3_w_inner.iuml": "' header comment\n@startuml\nKea -> Tui : inner\n@enduml\nKea -> Tui : not this\n",
 "cr3_w_vars.iuml": "@startuml\n!$who = \"Moa\"\n!procedure $hi()\nKea -> $who : hi\n!endprocedure\n@enduml\n",
 "cr3_w_end_bs.iuml": "@startuml\nKea -> Tui : a\n  \\enduml\nKea -> Tui : b\n",
 "cr3_w_end_trailing.iuml": "@startuml\nKea -> Tui : a\n@enduml trailing\nKea -> Tui : b\n",
 "cr3_w_bs_start.iuml": "\\startuml\nKea -> Tui : bs\n\\enduml\n",
 "cr3_w_mm.iuml": "* pre\n@startmindmap\n** child\n@endmindmap\n* post\n",
 "cr3_r_mid_end.iuml": "Kea -> Tui : a\n@enduml\nKea -> Tui : b\n",
 "cr3_tail.iuml": "Kea -> Tui : inc \\\n",
}
C = []
def add(name, src, valid, axes, pred): C.append((name, src, valid, axes, pred))

# --- A: typed TIM lines: each directive kind at the start, middle and end of the block, with a post-TIM tail ---
# Java: TLineType PLAIN lines (incl. !pragma and !assume, which are diagram commands) are TIM result lines;
# every other TIM directive (and comments) yields no entry. checkEndingBackslash joins over the TIM list.
D = [
 ("define", "!define FOO bar\n", "!define FOO bar\n", True),
 ("procedure", "!procedure $p()\nTui -> Kea : p\n!endprocedure\n", None, True),
 ("if", "!if 1\n!endif\n", "!if 0\nTui -> Kea : hidden\n!endif\n", True),
 ("include", "!include cr3_directives.iuml\n", None, True),
 ("theme", "!theme plain\n", None, "theme"),
 ("option", "!option language \"en\"\n", None, True),
 ("pragma", "!pragma layout smetana\n", None, False),
 ("assume", "!assume transparent light\n", None, False),
 ("log", "!log hello\n", None, True),
 ("dump", "!dump_memory\n", None, True),
 ("startsub", "!startsub SUBA\n!endsub\n", None, True),
]
for kind, lines, mid, consumed in D:
    mid = mid or lines
    first = lines.split("\n")[0]
    if consumed is True:
        ps = ("Welcome: the directive has no TIM entry; the tail absorbs the end line; getTotalLineCount()==2",
              "message 'x Tui -> Kea : y' (tail passes over the consumed directive)",
              "messages 'y' and 'x @enduml' (3 lines)")
    elif consumed is False:
        ps = (f"no Welcome: '{first}' is a PLAIN TIM line (count 3); message 'x @enduml'",
              f"message 'x {first}' then 'y' (tail joins the PLAIN directive line)",
              f"messages 'y' and 'x {first}'")
    else:
        ps = ("no Welcome: the theme's TIM lines count; message 'x @enduml' with theme plain",
              "tail joins the theme's first TIM line (pre-existing residue); 'y' renders",
              "tail joins the theme's first TIM line (pre-existing residue)")
    add(f"cr3_dir_{kind}_start", S + lines + T + E, True, ["typed-line", f"directive:{kind}", "position:start", "two-line"], ps[0])
    add(f"cr3_dir_{kind}_middle", S + T + mid + Y + E, True, ["typed-line", f"directive:{kind}", "position:middle"], ps[1])
    add(f"cr3_dir_{kind}_end", S + Y + T + lines + E, True, ["typed-line", f"directive:{kind}", "position:end"], ps[2])
add("cr3_dir_teoz_pragma_start", S + "!pragma teoz true\n" + T + E, True, ["typed-line", "directive:pragma", "teoz", "two-line"],
    "no Welcome: the pragma is a PLAIN TIM line; teoz message 'x @enduml'")

# --- B: the two-line rule at its edges ---
add("cr3_two_one_line_after_join", S + T + "@enduml \\\n", False, ["two-line", "count 1", "end continued"],
    "tail and continued end both pending; source is the start line only (count 1): empty-description error, not the Welcome factory")
add("cr3_two_zero_content_end_continued", S + "@enduml \\\n", False, ["two-line", "count 1", "end continued"],
    "count 1: empty-description error page")
add("cr3_two_end_continued_after_two", S + ONE + "Tui -> Kea : two %chr(92)\n@enduml \\\n", True, ["two-line", "count 2 with two content lines"],
    "Welcome: 'two \\' and the continued end are both pending and dropped; count 2")
add("cr3_two_comments_only", S + T + "' c1\n/' c2 '/\n/'\nc3\n'/\n" + E, True, ["two-line", "comments only"], "Welcome")
add("cr3_two_directives_only", S + "!$a = 1\n!define B 2\n" + T + "!$c = 3\n!undef B\n" + E, True, ["two-line", "directives only"], "Welcome")
add("cr3_two_blank_after_tail", S + T + "\n" + E, True, ["two-line", "count 3", "blank line is a TIM line"], "message 'x ' (3 lines)")
add("cr3_two_three_lines_before", S + Y + T + E, True, ["two-line", "count 3"], "messages 'y' and 'x @enduml'")
add("cr3_two_three_lines_after", S + T + Y + E, True, ["two-line", "count 3"], "message 'x Tui -> Kea : y'")
add("cr3_two_double_tail", S + "Kea -> Tui : a %chr(92)\nb %chr(92)\n" + E, True, ["two-line", "two tails"], "Welcome ('a b @enduml')")
add("cr3_two_activity_tail", S + ":act %chr(92)\n" + E, True, ["two-line", "activity text"], "Welcome (factory first)")
add("cr3_two_mindmap_control", "@startmindmap\n* r %chr(92)\n@endmindmap\n", True, ["two-line", "control", "no join outside @startuml"],
    "mindmap node 'r \\' (checkEndingBackslash false)")
add("cr3_two_nojoin_option_handwritten", S + "!option handwritten true\n" + E, True, ["two-line", "no join", "Rust-only line"],
    "Welcome: !option has no TIM entry, count 2")
add("cr3_two_nojoin_pragma", S + "!pragma layout smetana\n" + E, True, ["two-line", "no join", "PLAIN directive"],
    "count 3: no Welcome factory; Java's empty-sequence outcome")
add("cr3_two_include_tail_then_dump", S + "!include cr3_tail.iuml\n!dump_memory\n" + E, True, ["two-line", "dump", "include tail"],
    "Welcome: !dump_memory has no TIM entry")

# --- C: \end... forms (StartUtils#isEndDirective in BlockUmlBuilder; PSystemCommandFactory end peek) ---
add("cr3_end_bs_end_alone", S + ONE + "\\end\n" + TWO + E, True, ["end-form", "\\end"], "'one' only")
add("cr3_end_bs_endless_indented", S + ONE + "  \\endless story\n" + TWO + E, True, ["end-form", "\\end prefix", "indented"], "'one' only")
add("cr3_end_bs_endmindmap_in_uml", S + ONE + "\\endmindmap\n" + TWO + E, True, ["end-form", "other family end"], "'one' only")
add("cr3_end_bs_enduml_trailing", S + ONE + "\\enduml trailing words\n" + TWO + E, True, ["end-form", "trailing text"], "'one' only")
add("cr3_end_bs_End_case", S + ONE + "\\Enduml\n" + E, False, ["end-form", "case-sensitive"], "syntax error on '\\Enduml'")
add("cr3_end_double_backslash", S + ONE + "\\\\enduml\n" + E, False, ["end-form", "double backslash"], "syntax error")
add("cr3_end_bs_in_block_comment", S + ONE + "/'\n\\enduml\n'/\n" + TWO + E, True, ["end-form", "inside block comment"],
    "BlockUmlBuilder ends at \\enduml (comments unknown there); TIM's open comment eats the end; count 2 -> Welcome")
add("cr3_end_at_in_block_comment", S + ONE + "/'\n@enduml\n'/\n" + TWO + E, True, ["end-form", "inside block comment", "control @end"],
    "as the \\enduml case: Welcome")
add("cr3_end_bs_in_if0", S + ONE + "!if 0\n\\enduml\n!endif\n" + TWO + E, True, ["end-form", "inside inactive !if"],
    "block ends at \\enduml; the end line is inactive in TIM: count 2 -> Welcome (or a TIM error)")
add("cr3_end_at_in_if0", S + ONE + "!if 0\n@enduml\n!endif\n" + TWO + E, True, ["end-form", "inside inactive !if", "control @end"],
    "as the \\enduml case")
add("cr3_end_bs_in_procdef", S + "!procedure $p()\nKea -> Tui : p\n\\enduml\n!endprocedure\n$p()\n" + E, False,
    ["end-form", "inside procedure definition"], "block ends inside the definition: no call, count 1 or TIM error")
add("cr3_end_bs_in_note", S + "note over Kea\n\\endpoint\nend note\n" + E, False, ["end-form", "inside note body"],
    "block ends at '\\endpoint': unclosed note, syntax error")
add("cr3_end_us_before_bs_end", S + ONE + "\u001f\\enduml\n" + TWO + E, True, ["end-form", "U+001F is Character#isWhitespace"], "'one' only")
add("cr3_end_nel_before_bs_end", S + ONE + "\u0085\\enduml\n" + TWO + E, False, ["end-form", "U+0085 is not Character#isWhitespace"],
    "not an end directive: syntax error")
add("cr3_end_nbsp_before_bs_end", S + ONE + "\u00a0\\enduml\n" + E, False, ["end-form", "NBSP is not Character#isWhitespace"], "syntax error")
add("cr3_end_tim_chr64_enduml", S + ONE + "%chr(64)enduml\n" + TWO + E, True, ["end-form", "TIM-generated @enduml", "PSystemCommandFactory end peek"],
    "the command loop stops at the generated end: 'one' only")
add("cr3_end_tim_chr92_enduml", S + ONE + "%chr(92)enduml\n" + TWO + E, True, ["end-form", "TIM-generated \\enduml"], "'one' only")
add("cr3_end_mindmap_bs_end", "@startmindmap\n* a\n\\endmindmap\n* b\n@endmindmap\n", True, ["end-form", "mindmap family"], "root 'a' only")
add("cr3_end_pipe_second_diagram", S + ONE + "\\enduml\n@startuml\n" + TWO + E, True, ["end-form", "second diagram in Pipe text"], "'one' only")
add("cr3_end_bs_end_no_at_end", S + ONE + "\\enduml\n", True, ["end-form", "Pipe appends @enduml"], "'one'")

# --- D: wrapped includes (DiagramDetector#extractFromFile) ---
add("cr3_wrap_three_blocks", S + "!include cr3_w_three.iuml\n" + E, True, ["wrapped", "multiple blocks"], "'first' only")
add("cr3_wrap_three_blocks_bang2", S + "!include cr3_w_three.iuml!2\n" + E, True, ["wrapped", "!N suffix ignored for local files"], "'first' only")
add("cr3_wrap_ids_bang_beta", S + "!include cr3_w_ids.iuml!beta\n" + E, True, ["wrapped", "!id suffix ignored for local files"], "'alpha' only")
add("cr3_wrap_first_is_mindmap", S + "!include cr3_w_mixed.iuml\n" + E, False, ["wrapped", "first start of any type"],
    "'* mm' extracted into the sequence: syntax error")
add("cr3_wrap_hash_header", S + "!include cr3_w_hash.iuml\n" + E, True, ["wrapped", "header ##", "prefix line becomes empty"], "'hashed'")
add("cr3_wrap_angle_header", S + "!include cr3_w_angle.iuml\n" + E, True, ["wrapped", "header <x>"], "'angled'")
add("cr3_wrap_superscript_header", S + "!include cr3_w_sup.iuml\n" + E, True, ["wrapped", "header char not isLetterOrDigit (U+00B2)"], "'sup'")
add("cr3_wrap_bom_start", S + "!include cr3_w_bom.iuml\n" + TWO + E, True, ["wrapped", "BOM"], "'bom' then 'two'")
add("cr3_wrap_bom_hash_header", S + "!include cr3_w_bom_hash.iuml\n" + E, True, ["wrapped", "BOM", "header #"], "'bomhash' (ReadLineReader strips the BOM first)")
add("cr3_wrap_bom_inner_line", S + "!include cr3_w_bom_lines.iuml\n" + E, True, ["wrapped", "BOM on a body line"], "'midbom' (every line's BOM is stripped)")
add("cr3_raw_bom_include", S + "!include cr3_r_bom.iuml\n" + E, True, ["raw include", "BOM"], "'rawbom'")
add("cr3_wrap_crlf_continuation_comments", S + "!include cr3_w_crlf_cont.iuml\n" + E, True, ["wrapped", "CRLF", "continuation skips comments"], "'c1 c2'")
add("cr3_wrap_tail_runs_into_second_block", S + "!include cr3_w_tail_second.iuml\n" + E, False, ["wrapped", "merge swallows end", "nested start in body"],
    "body is 'x @enduml', '@startuml', 'y': the '@startuml' TIM line is no command: syntax error")
add("cr3_wrap_nested_wrapped", S + "!include cr3_w_outer.iuml\n" + E, True, ["wrapped", "nested"], "'inner' then 'outer'")
add("cr3_wrap_vars_and_procedure", S + "!include cr3_w_vars.iuml\n$hi()\n" + E, True, ["wrapped", "definitions"], "'hi' to Moa")
add("cr3_wrap_backslash_end", S + "!include cr3_w_end_bs.iuml\n" + E, True, ["wrapped", "\\enduml end"], "'a' only")
add("cr3_wrap_end_trailing_text", S + "!include cr3_w_end_trailing.iuml\n" + E, True, ["wrapped", "end with trailing text"], "'a' only")
add("cr3_wrap_backslash_start", S + "!include cr3_w_bs_start.iuml\n" + E, True, ["wrapped", "\\startuml"], "'bs'")
add("cr3_wrap_mindmap_family", "@startmindmap\n* root\n!include cr3_w_mm.iuml\n@endmindmap\n", True, ["wrapped", "mindmap family"], "root with child; pre/post dropped")
add("cr3_raw_include_mid_end", S + "!include cr3_r_mid_end.iuml\n" + TWO + E, True, ["raw include", "included @enduml", "PSystemCommandFactory end peek"],
    "the command loop stops at the included @enduml: 'a' only")

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
