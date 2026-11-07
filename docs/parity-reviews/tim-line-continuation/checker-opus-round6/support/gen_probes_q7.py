"""Checker round-6 (Opus) held-out probes, batch Q7, for T14.22 round 6 (maker tip 7e13632cd, code 59d78989f).
Written before any Java result for these inputs exists. Java predictions come from the pinned source (71806a237):
  - Pipe#readSingleDiagram (Pipe.java:180-225): a literal column-0 '@start' is the mark and lines before it are discarded;
    with no mark, '@@@format ' lines are never appended, leading lines whose String#trim is empty are skipped
    (NO_CONTENT), and the text is "@startuml\n" + sb + "@enduml\n". data-source-line is the 0-based LineLocation position.
  - BlockUmlBuilder (91-135) reads UncommentReadLine(ReadLineReader) through Preprocessor (ReadFilterMergeLines);
    every isStartDirective line restarts the block, the first isEndDirective line closes it.
  - UncommentReadLine#readLine (57-81) + StartUtils#beforeStartUml (56-80): a line whose prefix before '[@\\]start'
    has no letter/digit/'_'/'~' (outside <...>) sets headerToRemove; later lines that start with it lose it; a line
    that is a prefix of it becomes "". The directive test runs before the <...> test.
  - DiagramType#findStartTypes/getTypes (69-216): Character#isWhitespace, '@' or '\\', 'start' (ignore case), then a
    switch on Character.toLowerCase and regionMatches(true) prefix tests; else UNKNOWN -> PSystemUnsupported.
    regionMatches(true) also folds non-ASCII (U+0131 dotless i upper-cases to 'I').
  - Character#isWhitespace excludes U+00A0, U+2007, U+202F; String#trim strips <= U+0020; StringUtils#trin strips only
    space, tab, CR, LF, NUL.
intended_valid is the checker's Java-validity prediction; 'unsure' is marked in axes.
Usage: python3 -I gen_probes_q7.py OUT_DIR"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
B = "\\"
NB, FIG, NNB, ZW, BOM, IDEO, US = " ", " ", " ", "​", "﻿", "　", "\u001f"
RES = {
 "q7_parts.iuml": "participant Plover\n",
 "q7_nbsp_part.iuml": f"{NB}@startuml\n{NB}participant Plover\n{NB}@enduml\n",
 "q7_raw_part.iuml": "participant Plover\nPlover -> Kestrel : inc\n",
}
C = []
def case(name, text, valid, axes, crlf=False):
    C.append(dict(name=name, text=text, intended_valid=valid, axes=axes, crlf=crlf))
# --- Pipe numbering, no column-0 '@start' mark ---
case("q7_num_bs_seq_cont", f"{B}startuml\nKestrel -> Marten : one {B}\ntwo\nMarten -> Plover : three\n{B}enduml\n", True, ["numbering", "no mark", "continuation", "sequence"])
case("q7_num_bs_class_cont", f"{B}startuml\nclass Kestrel {B}\n{{\n  +wing : int\n}}\nclass Marten\nKestrel --> Marten\n{B}enduml\n", True, ["numbering", "no mark", "continuation", "class"])
case("q7_num_bs_state_cont", f"{B}startuml\n[*] --> Roost\nRoost --> Glide : dusk {B}\nfall\nGlide --> [*]\n{B}enduml\n", True, ["numbering", "no mark", "continuation", "state"])
case("q7_num_ideo_blank_bs_class", f"{IDEO}\n{B}startuml\nclass Sedge\nclass Rush\nSedge --> Rush\n{B}enduml\n", True, ["numbering", "U+3000-only first line is content for String#trim"])
case("q7_num_nbsp_blank_bs_seq", f"{NB}\n{B}startuml\nKestrel -> Marten : nb\n{B}enduml\n", True, ["numbering", "NBSP-only first line is content for String#trim"])
case("q7_num_us_blank_bs_seq", f"{US}\n{B}startuml\nKestrel -> Marten : us\n{B}enduml\n", True, ["numbering", "U+001F-only first line is blank for String#trim"])
case("q7_num_format_mid_class", f"{B}startuml\nclass Sedge\n@@@format svg\nclass Rush\nSedge --> Rush\n{B}enduml\n", True, ["numbering", "no mark", "@@@format between content lines"])
case("q7_num_format_mid_marked", "@startuml\nclass Sedge\n@@@format svg\nclass Rush\nSedge --> Rush\n@enduml\n", True, ["numbering", "marked control", "@@@format between content lines"])
case("q7_num_comment_blank_bs_obj", f"' heading\n\n{B}startuml\nobject Sedge\nobject Rush\nSedge --> Rush\n{B}enduml\n", True, ["numbering", "blank line after content seen is appended", "object"])
case("q7_num_crlf_tab_blank_bs_state", f"\t\n\n{B}startuml\n[*] --> Roost\nRoost --> [*]\n{B}enduml\n", True, ["numbering", "tab and blank leading lines", "state"], crlf=True)
case("q7_num_nomark_plain_class", "class Sedge\nclass Rush\nSedge --> Rush\n", True, ["numbering", "no start line at all", "class"])
case("q7_num_bs_usecase_blank_inside", f"{B}startuml\nactor Ranger\n\nusecase Patrol\n\nRanger --> Patrol\n{B}enduml\n", True, ["numbering", "no mark", "usecase"])
case("q7_num_bs_activity_if", f"{B}startuml\nstart\n:Gather reeds;\nif (dry?) then (yes)\n  :Weave;\nelse (no)\n  :Wait;\nendif\nstop\n{B}enduml\n", True, ["numbering", "no mark", "activity"])
case("q7_num_ws_at_component", "  @startuml\n[Pump] --> [Valve]\ncomponent Gauge\n@enduml\n", True, ["numbering", "no mark", "component"])
case("q7_num_inc_before_bs", f"!include q7_parts.iuml\n{B}startuml\nKestrel -> Marten : after\n{B}enduml\n", True, ["numbering", "include before \\start is in the discarded Pipe block"])
case("q7_num_def_before_bs", f"!$who = \"Plover\"\n{B}startuml\nKestrel -> Marten : $who\n{B}enduml\n", True, ["numbering", "variable before \\start is in the discarded Pipe block"])
case("q7_num_timing_bs", f"{B}startuml\nrobust \"Valve\" as V\nconcise \"Pump\" as P\n@0\nV is Open\nP is Idle\n@5\nV is Shut\nP is Busy\n{B}enduml\n", True, ["numbering", "no mark", "timing"])
case("q7_num_gantt_blank_bs", f"\n{B}startgantt\n[Dredge] lasts 4 days\n[Pile] starts at [Dredge]'s end and lasts 2 days\n{B}endgantt\n", True, ["numbering", "no mark", "gantt"])
case("q7_num_ws_json", "  @startjson\n{\"sedge\": [1, 2], \"rush\": {\"a\": true}}\n@endjson\n", True, ["numbering", "no mark", "json"])
case("q7_num_bs_wbs_cont", f"{B}startwbs\n* Reedbed {B}\nnorth\n** Ditch\n{B}endwbs\n", True, ["numbering", "no mark", "wbs", "continuation"])
case("q7_num_bs_mindmap_crlf", f"{B}startmindmap\n* Delta\n** Channel\n*** Bar\n{B}endmindmap\n", True, ["numbering", "no mark", "mindmap"], crlf=True)
case("q7_num_tim_func_bs", f"{B}startuml\n!function $tag($x)\n!return \"<\" + $x + \">\"\n!endfunction\nKestrel -> Marten : $tag(\"a\")\nMarten -> Kestrel : $tag(\"b\")\n{B}enduml\n", True, ["numbering", "no mark", "TIM function"])
case("q7_num_tim_if_bs_class", f"{B}startuml\n!$on = 1\n!if $on\nclass Sedge\n!else\nclass Rush\n!endif\nclass Reed\nSedge --> Reed\n{B}enduml\n", True, ["numbering", "no mark", "TIM if", "class"])
case("q7_num_inc_raw_crlf", f"{B}startuml\n!include q7_raw_part.iuml\nKestrel -> Marten : raw\n{B}enduml\n", True, ["numbering", "no mark", "raw include"], crlf=True)
case("q7_num_inc_nbsp_wrapped", f"{B}startuml\n!include q7_nbsp_part.iuml\nKestrel -> Marten : main\n{B}enduml\n", True, ["numbering", "no mark", "wrapped include with NBSP header"])
# --- errors ---
case("q7_err_bs_class_line", f"{B}startuml\nclass Sedge\nthis line ( is ]] not valid\n{B}enduml\n", False, ["error", "no mark", "line number on error page"])
case("q7_err_nbsp_hdr_seq", f"{NB}@startuml\nKestrel -> Marten : a\n{NB}this line ( is ]] not valid\n{NB}@enduml\n", False, ["error", "NBSP header", "line number on error page"])
# --- DiagramType#getTypes ---
case("q7_type_bs_regexp", f"{B}startregexp\n[0-9]{{2}}k\n{B}endregexp\n", True, ["start type", "prefix regex"])
case("q7_type_bs_ebnfx", f"{B}startebnfx\nreed = \"r\" , {{ \"e\" }} ;\n{B}endebnfx\n", True, ["start type", "prefix ebnf"])
case("q7_type_at_salt_mixed", "@startSalt\n{\n  Login | \"kestrel\"\n  [ OK ]\n}\n@endsalt\n", True, ["start type", "mixed-case salt; Pipe expects @endSalt"])
case("q7_type_bs_nwdiagram", f"{B}startnwdiagram\nnwdiag {{\n  network dmz {{\n    gate;\n  }}\n}}\n{B}endnwdiagram\n", True, ["start type", "prefix nwdiag"])
case("q7_type_bs_yamlfile", f"{B}startyamlfile\nreed: 4\nsedge: [1, 2]\n{B}endyaml\n", True, ["start type", "prefix yaml"])
case("q7_type_bs_umm", f"{B}startumm\nKestrel -> Marten : u\n{B}enduml\n", False, ["start type", "u but not uml: UNKNOWN"])
case("q7_type_bare_bs_start", f"{B}start\nKestrel -> Marten : bare\n{B}enduml\n", False, ["start type", "bare \\start is not a start directive (p >= length)", "unsure"])
case("q7_type_at_tab_uml", "@start\tuml\nKestrel -> Marten : tab\n@enduml\n", False, ["start type", "tab after start: UNKNOWN"])
case("q7_type_bs_umlupper_diag", f"{B}startUMLdiagram\nKestrel -> Marten : big\n{B}enduml\n", True, ["start type", "upper-case prefix uml"])
case("q7_type_bs_dotless_mindmap", f"{B}startmındmap\n* Sedge\n** Rush\n{B}endmindmap\n", True, ["start type", "regionMatches(true) folds U+0131 to I", "unsure"])
case("q7_type_inner_jsonx", "@startuml\n@startjsonx\n{\"reed\": 3}\n@endjson\n@enduml\n", True, ["start type", "inner prefix json restarts the block"])
case("q7_type_inner_quote_mindmap", "@startuml\n'@startmindmap\n* Sedge\n** Rush\n@enduml\n", True, ["start type", "commented inner start becomes a start through UncommentReadLine"])
case("q7_type_bs_project_upper", f"{B}startPROJECT\n[Caulk deck] lasts 2 days\n{B}endproject\n", True, ["start type", "upper-case project -> GANTT"])
case("q7_type_bs_wbs_name", f"{B}startwbs reedbed\n* Reedbed\n** North\n** South\n{B}endwbs\n", True, ["start type", "wbs with a name"])
# --- UncommentReadLine headers and invisible prefixes ---
case("q7_hdr_figsp_at", f"{FIG}@startuml\nKestrel -> Marten : fig\n@enduml\n", True, ["header", "U+2007 is not Java whitespace"])
case("q7_hdr_nnbsp_bs_class", f"{NNB}{B}startuml\nclass Sedge\n{NNB}class Rush\nSedge --> Rush\n{NNB}{B}enduml\n", True, ["header", "U+202F header stripped from content and end"])
case("q7_hdr_quote_start", "'@startuml\n'Kestrel -> Marten : quoted\n'@enduml\n", True, ["header", "quote header uncomments every line"])
case("q7_hdr_quote_comment_live", "'@startuml\nKestrel -> Marten : a\n' plain remark\n'@enduml\n", False, ["header", "quote header turns a comment into content"])
case("q7_hdr_quote_continuation", "'@startuml\nKestrel -> Marten : one \\\n' two\n'@enduml\n", True, ["header", "continuation reads the uncommented line"])
case("q7_hdr_tag_continuation", "<q>@startuml\n<q>Kestrel -> Marten : one \\\n<q>two\n<q>@enduml\n", True, ["header", "<...> header", "continuation"])
case("q7_hdr_nbsp_end_only", "@startuml\nKestrel -> Marten : e\n @enduml\n", False, ["header", "NBSP before the end only: content line", "unsure"])
case("q7_hdr_nbsp_tab", f"{NB}\t@startuml\nKestrel -> Marten : nt\n{NB}@enduml\n", True, ["header", "NBSP then tab"])
case("q7_hdr_zwsp_class", f"{ZW}@startuml\n{ZW}class Sedge\n{ZW}class Rush\n{ZW}Sedge --> Rush\n{ZW}@enduml\n", True, ["header", "U+200B header", "class"])
case("q7_hdr_bom_content", f"@startuml\n{BOM}Kestrel -> Marten : bom\n@enduml\n", True, ["header", "BOM on a content line (ReadLineReader)"])
case("q7_hdr_nbsp_state", f"{NB}{B}startuml\n{NB}[*] --> Roost\n{NB}Roost --> [*]\n{NB}{B}enduml\n", True, ["header", "NBSP", "state", "numbering"])
case("q7_hdr_tilde", "~@startuml\nKestrel -> Marten : t\n@enduml\n", False, ["header", "'~' is isWordOrTilde: no header, content line"])
case("q7_hdr_dash_space", "- @startuml\n- Kestrel -> Marten : d\n-@enduml\n", True, ["header", "header '-' before a spaced start"])
case("q7_hdr_nested_angle", "<a><b>@startuml\n<a><b>Kestrel -> Marten : ab\n<a><b>@enduml\n", True, ["header", "two <...> groups"])
case("q7_hdr_angle_unclosed", "<a @startuml\n<a Kestrel -> Marten : open\n<a @enduml\n", True, ["header", "directive test runs before the unclosed <...> test"])
# --- marked controls ---
case("q7_ctl_marked_seq_cont", "@startuml\nKestrel -> Marten : a \\\nb\nMarten -> Kestrel : c\n@enduml\n", True, ["control", "marked", "continuation"])
case("q7_ctl_marked_class_crlf", "@startuml\nclass Sedge\nclass Rush\nSedge --> Rush\n@enduml\n", True, ["control", "marked", "class"], crlf=True)
for name, text in RES.items(): (out / name).write_text(text, encoding="utf-8", newline="")
for c in C:
    t = c["text"].replace("\n", "\r\n") if c["crlf"] else c["text"]
    (out / f"{c['name']}.puml").write_bytes(t.encode("utf-8"))
(out / "cases.json").write_text(json.dumps([{k: v for k, v in c.items() if k != "text"} for c in C], indent=1, ensure_ascii=False))
(out / "resources.json").write_text(json.dumps(sorted(RES), indent=1))
print(len(C), sum(c["intended_valid"] for c in C), len(RES))
