"""Checker round-5 recheck (Opus) held-out probes, batch R5B, for T14.22 round 5 (maker tip 6fd071169, code ea5375293).
Written before any Java result for these inputs exists. Predictions from the pinned Java source (71806a237):
  - Pipe#readSingleDiagram (Pipe.java:180-225): column-0 literal '@start' is the mark; lines before it are discarded
    (sb.setLength(0)); with no mark, leading blank lines (trim().length()==0) are skipped (NO_CONTENT) and the text is
    "@startuml\n" + rest + "@enduml\n". data-source-line is the 0-based LineLocation position in that text.
  - BlockUmlBuilder (100-135): every StartUtils#isStartDirective line restarts the block; the first isEndDirective
    line closes it; an end with no open block is ignored.
  - DiagramType#findStartTypes (69-92): Character#isWhitespace, '@' or '\\', regionMatches(true) 'start', then
    getTypes: check(key) is a case-insensitive PREFIX test ('umlish' is uml); any other char is UNKNOWN (still a start)
    and PSystemBuilder returns PSystemUnsupported for UNKNOWN.
  - StartUtils#isEndDirective (110): case-sensitive 'end' after whitespace and '@'/'\\' ('\\endless' ends a block).
  - ReadLineReader#readLine (99): a leading U+FEFF is removed from every line (after Pipe has looked at it).
  - DiagramExtractor (55-100): '!include f!N' / '!include f!ID' select blocks by isStartDirective (and 'id=ID').
intended_valid is the checker's Java-validity prediction; 'unsure' is marked in axes.
Usage: python3 -I gen_probes_r5b.py OUT_DIR"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
B = "\\"
RES = {
 "r5b_blocks.iuml": f"{B}startuml\nOtter -> Badger : zero\n{B}enduml\n{B}startuml\nBadger -> Otter : one\n{B}enduml\n",
 "r5b_ids.iuml": f"{B}startuml(id=REEF)\nOtter -> Badger : reef\n{B}enduml\n{B}startuml(id=TIDE)\nBadger -> Otter : tide\n{B}enduml\n",
 "r5b_subsrc.iuml": f"{B}startuml\n!startsub GULL\nOtter -> Badger : gull\n!endsub\n{B}enduml\n",
}
C = []
def case(name, text, valid, axes, crlf=False):
    C.append(dict(name=name, text=text, intended_valid=valid, axes=axes, crlf=crlf))
SEQ3 = "Otter -> Badger : one\nBadger -> Heron : two\nHeron -> Otter : three\n"
# --- data-source-line numbering when Pipe finds no column-0 '@start' mark (Java: input line n is position n) ---
case("r5b_num_bs_seq_three", f"{B}startuml\n{SEQ3}{B}enduml\n", True, ["numbering", "\\start sequence, 3 messages"])
case("r5b_num_ws_at_seq", f"   @startuml\nparticipant Otter\nOtter -> Badger : spaced\n@enduml\n", True, ["numbering", "3-space indented @startuml"])
case("r5b_num_upper_at_seq", f"@StartUml\nOtter -> Badger : upper\nBadger --> Otter : back\n@enduml\n", True, ["numbering", "@StartUml (no Pipe mark)"])
case("r5b_num_emsp_seq", f" @startuml\nOtter -> Badger : emsp\n@enduml\n", True, ["numbering", "U+2003 before @startuml (Java whitespace)"])
case("r5b_num_us_bs_seq", f"\u001f{B}startuml\nOtter -> Badger : us\n{B}enduml\n", True, ["numbering", "U+001F before \\startuml (Java whitespace)", "unsure"])
case("r5b_num_two_blank_bs_seq", f"\n\n{B}startuml\nOtter -> Badger : blank2\n{B}enduml\n", True, ["numbering", "two leading blank lines skipped by Pipe NO_CONTENT"])
case("r5b_num_comment_bs_seq", f"' lead note\n{B}startuml\nOtter -> Badger : cmt\n{B}enduml\n", True, ["numbering", "comment line before \\startuml"])
case("r5b_num_comment_at_seq", f"' lead note\n@startuml\nOtter -> Badger : cmt\n@enduml\n", True, ["numbering", "control: Pipe discards the line before its mark"])
case("r5b_num_bs_blank_inside", f"{B}startuml\n\nOtter -> Badger : a\n\n\nBadger -> Otter : b\n{B}enduml\n", True, ["numbering", "blank lines inside a \\start block"])
case("r5b_num_unmarked_bs_end_only", f"Otter -> Badger : a\nBadger -> Otter : b\n{B}enduml\n", True, ["numbering", "no start, \\enduml only"])
case("r5b_num_bs_activity", f"{B}startuml\nstart\n:Heron;\n:Wren;\nstop\n{B}enduml\n", True, ["numbering", "\\start activity"])
case("r5b_num_bs_state", f"{B}startuml\n[*] --> Calm\nCalm --> Storm\nStorm --> [*]\n{B}enduml\n", True, ["numbering", "\\start state"])
case("r5b_num_bs_object", f"{B}startuml\nobject Lantern\nobject Wick\nLantern --> Wick\n{B}enduml\n", True, ["numbering", "\\start object"])
case("r5b_num_bs_deployment", f"{B}startuml\nnode Quay\ndatabase Ledger\nQuay --> Ledger\n{B}enduml\n", True, ["numbering", "\\start deployment"])
case("r5b_num_tab_at_state", f"\t@startuml\n[*] --> Idle\nIdle --> [*]\n@enduml\n", True, ["numbering", "tab-indented @startuml state"])
case("r5b_num_pipe_nostart_blank_ws", f"\n   \n\t\nOtter -> Badger : x\nBadger -> Otter : y\n", True, ["numbering", "no start; blank/space/tab lines skipped (NO_CONTENT)"])
case("r5b_num_startsub_in_bs_main", f"{B}startuml\n!startsub KELP\nOtter -> Badger : kelp\n!endsub\nBadger -> Otter : after\n{B}enduml\n", True, ["numbering", "!startsub in a \\start main block"])
# --- start recognition ---
case("r5b_start_upper_type_mindmap", "@startMINDMAP\n* Orchard\n** Pear\n** Quince\n@endmindmap\n", True, ["start", "type upper case, Pipe expects @endMINDMAP"])
case("r5b_start_bs_yaml", f"{B}startyaml\nfruit: pear\ncount: 3\n{B}endyaml\n", True, ["start", "\\startyaml"])
case("r5b_start_bs_nwdiag", f"{B}startnwdiag\nnwdiag {{\n  network lan {{\n    web01;\n    web02;\n  }}\n}}\n{B}endnwdiag\n", True, ["start", "\\startnwdiag"])
case("r5b_start_bs_regex", f"{B}startregex\n[a-c]+x?\n{B}endregex\n", True, ["start", "\\startregex"])
case("r5b_start_bs_chen", f"{B}startchen\nentity HARBOUR {{\n  Name : TEXT\n}}\n{B}endchen\n", True, ["start", "\\startchen"])
case("r5b_start_bs_files", f"{B}startfiles\n/docs/readme.md\n/src/main.rs\n{B}endfiles\n", True, ["start", "\\startfiles"])
case("r5b_start_bs_creole", f"{B}startcreole\n**bold** and //italic// tide\n{B}endcreole\n", True, ["start", "\\startcreole"])
case("r5b_start_bs_project", f"{B}startproject\n[Paint hull] lasts 3 days\n[Dry hull] lasts 2 days\n{B}endproject\n", True, ["start", "\\startproject (GANTT)"])
case("r5b_start_bs_json_mixed", f"{B}StartJSON\n{{\"tide\": 1, \"reef\": [2, 3]}}\n@endjson\n", True, ["start", "\\StartJSON, @endjson"])
case("r5b_start_bs_umlish", f"{B}startumlish\nOtter -> Badger : ish\n{B}enduml\n", True, ["start", "check('uml') is a prefix test: umlish is UML"])
case("r5b_start_at_umlx", "@startumlx\nOtter -> Badger : x\n@enduml\n", True, ["start", "@startumlx is UML; Pipe expects @endumlx and appends it"])
case("r5b_start_paren_id", "@startuml(id=Gull)\nOtter -> Badger : id\n@enduml\n", True, ["start", "@startuml(id=...)"])
case("r5b_start_bs_name", f"{B}startuml harbour\nOtter -> Badger : named\n{B}enduml\n", True, ["start", "\\startuml name"])
case("r5b_start_bs_space_uml", f"{B}start uml\nOtter -> Badger : x\n{B}enduml\n", False, ["start", "'\\start uml': UNKNOWN type -> PSystemUnsupported"])
case("r5b_start_at_space_uml", "@start uml\nOtter -> Badger : x\n@enduml\n", False, ["start", "'@start uml': UNKNOWN type -> PSystemUnsupported"])
case("r5b_start_at_bare", "@start\nOtter -> Badger : x\n@end\n", False, ["start", "'@start' alone: Pipe mark, no BlockUml -> no valid start"])
case("r5b_start_nbsp", f" @startuml\nOtter -> Badger : x\n@enduml\n", False, ["start", "NBSP is not Java whitespace: content line", "unsure"])
case("r5b_start_bom", f"﻿@startuml\nOtter -> Badger : bom\n@enduml\n", True, ["start", "BOM: no Pipe mark; ReadLineReader strips it; restart"])
case("r5b_start_ideosp_bs_mindmap", f"　{B}startmindmap\n* Orchard\n** Pear\n{B}endmindmap\n", True, ["start", "U+3000 before \\startmindmap"])
case("r5b_start_in_note", "@startuml\nnote over Otter\n\\started here\nend note\n@enduml\n", False, ["start", "note line '\\started' restarts the block: UNKNOWN"])
# --- end recognition ---
case("r5b_end_other_suffix", "@startuml\nOtter -> Badger : x\n@endfoo\n", True, ["end", "@endfoo closes the block"])
case("r5b_end_bs_bare", f"{B}startuml\nOtter -> Badger : x\n{B}end\n", True, ["end", "\\end alone closes"])
case("r5b_end_mixed_case", "@startuml\nOtter -> Badger : x\n@Enduml\n", False, ["end", "@Enduml is content (case-sensitive end)"])
case("r5b_end_in_note", "@startuml\nnote over Otter\n  \\endless tide\nend note\n@enduml\n", False, ["end", "'\\endless' closes the block inside a note"])
case("r5b_end_without_start_tail", "Otter -> Badger : x\n@enduml\nBadger -> Otter : y\n", True, ["end", "no start; first @enduml closes; tail ignored"])
case("r5b_end_first_then_content", "@enduml\nOtter -> Badger : x\n", True, ["end", "Pipe's @startuml + input @enduml: empty block (Welcome)", "unsure"])
case("r5b_empty_bs_block", f"{B}startuml\n' nothing here\n{B}enduml\n", True, ["end", "empty \\start block (Welcome)"])
case("r5b_mix_at_bs_end_tail", f"@startuml\nOtter -> Badger : x\n{B}enduml\nBadger -> Otter : tail\n@enduml\n", True, ["end", "\\enduml closes before Pipe's end"])
# --- multiple and nested blocks ---
case("r5b_multi_at_then_bs_mindmap", f"@startuml\nOtter -> Badger : first\n@enduml\n{B}startmindmap\n* second\n{B}endmindmap\n", True, ["blocks", "Pipe stops at @enduml"])
case("r5b_multi_bs_then_bs_mindmap", f"{B}startuml\nOtter -> Badger : first\n{B}enduml\n{B}startmindmap\n* second\n{B}endmindmap\n", True, ["blocks", "first of two \\ blocks"])
case("r5b_multi_bs_mindmap_then_uml", f"{B}startmindmap\n* Orchard\n** Pear\n{B}endmindmap\n{B}startuml\nOtter -> Badger : second\n{B}enduml\n", True, ["blocks", "mindmap first"])
case("r5b_nest_uml_bs_mindmap", f"@startuml\n{B}startmindmap\n* Orchard\n** Pear\n{B}endmindmap\n@enduml\n", True, ["nested", "restart at \\startmindmap"])
case("r5b_nest_uml_content_bs_json", f"@startuml\nOtter -> Badger : lost\n{B}startjson\n{{\"k\": 2}}\n{B}endjson\n@enduml\n", True, ["nested", "restart discards the sequence line"])
# --- includes ---
case("r5b_inc_block_index", "@startuml\n!include r5b_blocks.iuml!1\n@enduml\n", True, ["include", "!include f!1 over \\start blocks"])
case("r5b_inc_block_id", "@startuml\n!include r5b_ids.iuml!TIDE\n@enduml\n", True, ["include", "!include f!ID over \\startuml(id=...)"])
case("r5b_inc_sub_from_bs", "@startuml\n!includesub r5b_subsrc.iuml!GULL\n@enduml\n", True, ["include", "!includesub from a \\start file"])
# --- error line numbers (manual comparison) ---
case("r5b_err_bs_seq_line", f"{B}startuml\nOtter -> Badger : ok\nthis is not (( valid\n{B}enduml\n", False, ["error-line", "\\start: Java numbers from Pipe's prepended line"])
case("r5b_err_ws_at_seq_line", "  @startuml\nOtter -> Badger : ok\n%%% broken\n@enduml\n", False, ["error-line", "indented @startuml"])
# --- CRLF ---
case("r5b_crlf_bs_seq", f"{B}startuml\nOtter -> Badger : crlf\nBadger -> Otter : back\n{B}enduml\n", True, ["crlf", "\\start sequence"], crlf=True)
case("r5b_crlf_bs_mindmap", f"{B}startmindmap\n* Orchard\n** Pear\n{B}endmindmap\n", True, ["crlf", "\\startmindmap"], crlf=True)
for c in C:
    t = c.pop("text"); 
    if c["crlf"]: t = t.replace("\n", "\r\n")
    (out / f"{c['name']}.puml").write_bytes(t.encode())
for p, t in RES.items(): (out / p).write_bytes(t.encode())
(out / "cases.json").write_text(json.dumps(C, indent=1, ensure_ascii=False))
(out / "resources.json").write_text(json.dumps(sorted(RES), indent=1))
print(len(C), sum(c["intended_valid"] for c in C), len(RES))
