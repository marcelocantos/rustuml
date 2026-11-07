"""Checker round-2 held-out probes (batch CR2) for T14.22 round 2 (maker tip e3aa1c6b4, code f2d12d0ef).
Written before any Java result for these inputs exists. intended_valid is the checker's Java prediction
from the pinned source trace (Pipe#readSingleDiagram, ReadFilterMergeLines, BlockUmlBuilder, UmlSource#loadInternal);
validity is decided later from raw Java SVG + stderr only.
Usage: python3 -I gen_probes.py OUT_DIR"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
RES = {
    "cr2_tail.iuml": "Kea -> Tui : tail \\\n",
    "cr2_tail_crlf.iuml": "Kea -> Tui : tail \\\r\n",
    "cr2_wrapped.iuml": "@startuml\nKea -> Tui : wrapped \\\n@enduml\n",
    "cr2_deep.iuml": "Kea -> Tui : deep \\\n",
    "cr2_mid.iuml": "!include cr2_deep.iuml\n",
    "cr2_mid_more.iuml": "!include cr2_deep.iuml\nTui -> Kea : mid\n",
    "cr2_note.iuml": "' nothing but a comment\n",
    "cr2_mm_tail.iuml": "** Branch \\\n",
    "cr2_comp_tail.iuml": "[Alder] --> \\\n",
    "cr2_act_tail.iuml": ":first \\\n",
}
S = "@startuml\n"; E = "@enduml\n"
# (name, source, intended_valid (Java prediction), axes, java_prediction)
C = [
 # --- A: Pipe start/end recognition (Pipe.java:180-224) ---
 ("cr2_seq_named_start_col0_endother_swallow", "@startuml gull\nWren -> Finch : one \\\n@endmindmap\nFinch -> Wren : two\n@enduml\n", False,
  ["pipe-end", "named start -> expectedEnd @end", "col-0 @endmindmap ends Pipe text"], "Pipe completes at @endmindmap (startsWith '@end'); the merge swallows it; no block -> 'No valid @start/@end'"),
 ("cr2_seq_exact_start_col0_endother_merged", S + "Wren -> Finch : one \\\n@endmindmap\nFinch -> Wren : two\n" + E, True,
  ["pipe-end", "exact @startuml -> expectedEnd @enduml", "@endmindmap merged into message"], "messages 'one @endmindmap' and 'two'"),
 ("cr2_seq_trailing_space_start_endother_swallow", "@startuml \nWren -> Finch : one \\\n@endmindmap\nFinch -> Wren : two\n@enduml\n", False,
  ["pipe-end", "start line with trailing space fails matches() -> @end"], "as named start: no block"),
 ("cr2_seq_crlf_exact_start_endother_merged", (S + "Wren -> Finch : one \\\n@endmindmap\nFinch -> Wren : two\n" + E).replace("\n", "\r\n"), True,
  ["pipe-end", "CRLF", "readLine strips CR so start matches exactly"], "same as the LF exact-start case"),
 ("cr2_seq_endumlx_col0_swallow", S + "Wren -> Finch : one \\\n@endumlx\nFinch -> Wren : two\n" + E, False,
  ["pipe-end", "startsWith(expectedEnd) prefix match"], "Pipe completes at @endumlx; merged; no block"),
 ("cr2_seq_startUML_case_end_merged", "@startUML\nWren -> Finch : late \\\n@enduml\n", True,
  ["pipe-end", "expectedEnd keeps start case (@endUML)", "case-insensitive start type"], "Pipe appends @endUML; message 'late @enduml'"),
 ("cr2_seq_tab_end_plain_control", S + "Wren -> Finch : one\n\t@enduml\n", True,
  ["control", "tab-indented end, no continuation"], "renders 'one'"),
 ("cr2_seq_after_end_continued_tail", S + "Wren -> Finch : one\n" + E + "Finch -> Wren : two \\\n" + E, True,
  ["pipe-end", "content after column-0 end"], "first diagram only ('one'); the rest has no start"),
 ("cr2_seq_two_blocks_first_swallowed", S + "Wren -> Finch : one \\\n" + E + S + "Finch -> Wren : two\n" + E, False,
  ["pipe-end", "multi-block", "first end swallowed"], "chunk 1 no block (error page), chunk 2 renders 'two' (T14.26 multi-block)"),
 ("cr2_seq_nested_start_in_continued_line", S + "Wren -> Finch : one \\\n" + S + "Finch -> Wren : two\n" + E, True,
  ["split-blocks", "start merged into continued line"], "messages 'one @startuml' and 'two'"),
 ("cr2_seq_nested_start_plain_control", S + "Wren -> Finch : one\n" + S + "Finch -> Wren : two\n" + E, True,
  ["control", "split-blocks", "BlockUmlBuilder resets at a nested start"], "renders 'two' only"),
 ("cr2_seq_start_line_continued", "@startuml \\\nWren -> Finch : one\nFinch -> Wren : two\n@enduml\n", True,
  ["pipe-end", "start line ends with backslash"], "start line '@startuml Wren -> Finch : one'; renders 'two'"),
 ("cr2_seq_end_line_continued_then_text", S + "Wren -> Finch : one\n@enduml \\\nFinch -> Wren : two\n", True,
  ["pipe-end", "end line ends with backslash"], "Pipe stops at the end line; renders 'one'"),
 ("cr2_seq_chained_continuation_indented_end", S + "Wren -> Finch : one \\\n   \\\n  @enduml\nFinch -> Wren : two\n" + E, True,
  ["pipe-end", "chained continuation into indented end"], "message 'one' + merged blanks + '@enduml', then 'two'"),
 ("cr2_seq_pre_start_backslash_text", "see C:\\\n" + S + "Wren -> Finch : one\n" + E, True,
  ["pipe-start", "text before start ends with backslash (Pipe discards it)"], "renders 'one'"),
 ("cr2_seq_pre_start_blank_then_continued", "\n\n" + S + "Wren -> Finch : one \\\n  @enduml\n", True,
  ["pipe-start", "blank lines before start"], "message 'one   @enduml', lines counted from @startuml"),
 # --- Z: empty / whitespace-only stdin ---
 ("cr2_empty_zero_bytes", "", False, ["empty"], "Pipe NO_CONTENT: no diagram, no output"),
 ("cr2_newlines_only", "\n\n\n", False, ["empty", "newlines"], "NO_CONTENT"),
 ("cr2_whitespace_only", "  \t \n   \n", False, ["empty", "spaces and tab"], "NO_CONTENT"),
 ("cr2_crlf_blank_only", "\r\n\r\n", False, ["empty", "CRLF"], "NO_CONTENT"),
 ("cr2_vt_ff_only", "\x0b\x0c\n", False, ["empty", "VT/FF are <= U+0020 for String#trim"], "NO_CONTENT"),
 ("cr2_nbsp_only", "\u00a0\n", False, ["whitespace", "NBSP is not trimmed by String#trim"], "content: wrapped in @startuml/@enduml; empty/welcome diagram"),
 ("cr2_no_start_plain", "Wren -> Finch : one\n", True, ["pipe-start", "no start: Pipe wraps"], "renders 'one'"),
 ("cr2_no_start_continued", "Wren -> Finch : one \\\ntwo\n", True, ["pipe-start", "no start: Pipe wraps", "continuation"], "renders 'one two'"),
 # --- B: include-tail joins (UmlSource#loadInternal) ---
 ("cr2_seq_wrapped_include_tail_then_end", S + "!include cr2_wrapped.iuml\n" + E, True,
  ["include-tail", "included file has its own @startuml/@enduml", "tail joins main end"], "message 'wrapped @enduml'"),
 ("cr2_seq_wrapped_include_tail_then_msg", S + "!include cr2_wrapped.iuml\nTui -> Kea : two\n" + E, True,
  ["include-tail", "wrapped include"], "message 'wrapped Tui -> Kea : two'"),
 ("cr2_seq_depth2_tail_then_msg", S + "!include cr2_mid.iuml\nTui -> Kea : two\n" + E, True,
  ["include-tail", "depth 2"], "message 'deep Tui -> Kea : two'"),
 ("cr2_seq_depth2_tail_then_end", S + "!include cr2_mid.iuml\n" + E, True,
  ["include-tail", "depth 2", "tail joins end"], "message 'deep @enduml'"),
 ("cr2_seq_depth2_tail_then_depth1_line", S + "!include cr2_mid_more.iuml\nTui -> Kea : two\n" + E, True,
  ["include-tail", "depth 2 tail then depth-1 line"], "'deep Tui -> Kea : mid', 'two'"),
 ("cr2_seq_depth2_tail_no_end", S + "Tui -> Kea : one\n!include cr2_mid.iuml\n", True,
  ["include-tail", "depth 2", "Pipe-appended end"], "'one', 'deep @enduml'"),
 ("cr2_seq_tail_then_backslash_end", S + "!include cr2_tail.iuml\n\\enduml\n", True,
  ["include-tail", "\\enduml end line"], "message 'tail \\enduml' (Pipe appends @enduml after it)"),
 ("cr2_seq_tail_then_indented_end", S + "!include cr2_tail.iuml\n  @enduml\n", True,
  ["include-tail", "indented end line (not Pipe's end)"], "message 'tail   @enduml'"),
 ("cr2_seq_tail_then_theme", S + "!include cr2_tail.iuml\n!theme plain\nTui -> Kea : two\n" + E, True,
  ["include-tail", "theme", "maker residue 2"], "tail joins the theme's first TIM line"),
 ("cr2_seq_tail_then_procedure", S + "!procedure $p()\nTui -> Kea : p1\nTui -> Kea : p2\n!endprocedure\n!include cr2_tail.iuml\n$p()\n" + E, True,
  ["include-tail", "procedure output"], "'tail Tui -> Kea : p1', 'p2'"),
 ("cr2_seq_tail_then_comment_include", S + "!include cr2_tail.iuml\n!include cr2_note.iuml\nTui -> Kea : two\n" + E, True,
  ["include-tail", "comment-only include"], "'tail Tui -> Kea : two'"),
 ("cr2_seq_tail_then_block_comment", S + "!include cr2_tail.iuml\n/' block\n comment '/\nTui -> Kea : two\n" + E, True,
  ["include-tail", "multi-line block comment"], "'tail Tui -> Kea : two'"),
 ("cr2_seq_tail_crlf_include", S + "!include cr2_tail_crlf.iuml\nTui -> Kea : two\n" + E, True,
  ["include-tail", "CRLF include file"], "'tail Tui -> Kea : two'"),
 ("cr2_mindmap_include_tail_not_joined", "@startmindmap\n* Root\n!include cr2_mm_tail.iuml\n** Leaf\n@endmindmap\n", True,
  ["include-tail", "mindmap: no checkEndingBackslash"], "no post-TIM join: 'Branch \\' and 'Leaf' nodes"),
 ("cr2_component_include_tail_link", S + "!include cr2_comp_tail.iuml\n[Rowan]\n" + E, True,
  ["include-tail", "component"], "one link Alder --> Rowan"),
 ("cr2_activity_include_tail", S + "start\n!include cr2_act_tail.iuml\nsecond;\nstop\n" + E, True,
  ["include-tail", "activity"], "action 'first second'"),
 ("cr2_seq_chr92_tail_then_directive", S + "Kea -> Tui : tail %chr(92)\n!$v = 1\nTui -> Kea : two $v\n" + E, True,
  ["post-TIM join", "no include", "directive passed over"], "'tail Tui -> Kea : two 1'"),
 ("cr2_seq_chr92_tail_then_end", S + "Kea -> Tui : tail %chr(92)\n" + E, True,
  ["post-TIM join", "tail joins end"], "'tail @enduml'"),
 ("cr2_seq_procedure_emits_tail", S + "!procedure $p()\nKea -> Tui : p1\nKea -> Tui : p2 %chr(92)\n!endprocedure\n$p()\nTui -> Kea : two\n" + E, True,
  ["post-TIM join", "procedure output ends with backslash"], "'p1', 'p2 Tui -> Kea : two'"),
 # --- S: in-band placeholder collision (DIRECTIVE_PLACEHOLDER = U+001E 'RSP') ---
 ("cr2_seq_chr30_rsp_line", S + "Wren -> Finch : one\n%chr(30)RSP\n" + E, False,
  ["placeholder collision", "%chr(30)"], "syntax error: no command accepts the line"),
 ("cr2_seq_raw_rs_rsp_line", S + "Wren -> Finch : one\n\x1eRSP\n" + E, False,
  ["placeholder collision", "raw U+001E"], "syntax error"),
 ("cr2_seq_note_chr30_rsp", S + "Wren -> Finch : one\nnote over Wren\n%chr(30)RSP\nend note\n" + E, True,
  ["placeholder collision", "note body"], "note with text"),
 ("cr2_seq_note_plain_rsp_control", S + "Wren -> Finch : one\nnote over Wren\nRSP\nend note\n" + E, True,
  ["control", "note body"], "note 'RSP'"),
 # --- T: theme / procedure with continuation ---
 ("cr2_seq_theme_then_continued", S + "!theme plain\nWren -> Finch : one \\\ntwo\n" + E, True,
  ["theme", "continuation"], "'one two'"),
 ("cr2_seq_procedure_body_continued", S + "!procedure $p($x)\nWren -> Finch : $x \\\nmore\n!endprocedure\n$p(\"hi\")\n" + E, True,
  ["procedure", "continuation in body"], "'hi more'"),
 ("cr2_seq_procedure_call_continued", S + "!procedure $p($a, $b)\nWren -> Finch : $a $b\n!endprocedure\n$p(\"hi\", \\\n\"there\")\n" + E, True,
  ["procedure", "continued call line"], "'hi there'"),
 # --- E: ditaa exemption ---
 ("cr2_ditaa_indented_end_continued_row", "@startditaa\n+-----+\n| Oak \\\n|  ok |\n+-----+\n  @endditaa\n", True,
  ["ditaa", "indented end", "equivalence with cr2_ditaa_continued_row_ref"], "same PNG as the reference"),
 ("cr2_ditaa_case_start_continued_row", "@startDitaa\n+-----+\n| Oak \\\n|  ok |\n+-----+\n@endditaa\n", True,
  ["ditaa", "start case", "Pipe expects @endDitaa and appends it"], "same PNG as the reference"),
 ("cr2_ditaa_continued_row_ref", "@startditaa\n+-----+\n| Oak \\\n|  ok |\n+-----+\n@endditaa\n", True,
  ["ditaa", "reference for equivalence"], "PNG, row not joined"),
]
cases = []
for name, src, valid, axes, pred in C:
    data = src.encode("utf-8")
    (out / f"{name}.puml").write_bytes(data)
    cases.append(dict(name=name, intended_valid=valid, axes=axes, java_prediction=pred, crlf="\r\n" in src))
for p, t in RES.items(): (out / p).write_bytes(t.encode())
assert len({c["name"] for c in cases}) == len(cases)
(out / "cases.json").write_text(json.dumps(cases, indent=1, ensure_ascii=False))
(out / "resources.json").write_text(json.dumps(sorted(RES), indent=1))
print(len(cases), sum(c["intended_valid"] for c in cases), len(RES))
