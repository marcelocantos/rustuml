"""Generate held-out round-2 probes (batch r2) for a Java reference request.

Usage: python3 -I gen_probes.py OUT_DIR
Writes OUT_DIR/cases.json, OUT_DIR/<name>.puml and the include resources. No Java is run here.
Every case is outside the golden corpus; names, labels and shapes are fresh (renamed from tc1/mc1).
Each case carries its Java prediction (from the account's trace), registered before any Java reference exists.
"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
B = "\\"
cases = []
def case(name, body, axes, java_prediction, valid=True, crlf=False):
    text = body if body.endswith("\n") else body + "\n"
    data = text.replace("\n", "\r\n") if crlf else text
    (out / f"{name}.puml").write_bytes(data.encode())
    cases.append(dict(name=name, intended_valid=valid, crlf=crlf, axes=axes, java_prediction=java_prediction))

RESOURCES = {
    "r2_tail.iuml": f"Heron -> Egret : drift {B}\n",
    "r2_tails.iuml": f"Heron -> Egret : one {B}\nand {B}\n",
    "r2_plain.iuml": "Egret -> Heron : plain\n",
    "r2_link.iuml": f"Alder --> Rowan : tail {B}\n",
    "r2_comp.iuml": f"[Alder] --> [Rowan] : wire {B}\n",
}
for path, text in RESOURCES.items():
    (out / path).write_bytes(text.encode())

# Obligation 1: Pipe#readSingleDiagram ends only at a column-0 startsWith(expectedEnd) line.
case("r2_seq_tab_end_after_backslash", f"@startuml\nWren -> Finch : late {B}\n\t@enduml",
     ["sequence", "tab-indented @enduml after backslash", "Pipe column-0 end"], "message 'late \\t@enduml' at line 1")
case("r2_seq_backslash_end_after_backslash", f"@startuml\nWren -> Finch : late {B}\n{B}enduml",
     ["sequence", "\\enduml after backslash", "Pipe column-0 end"], "message 'late \\enduml' at line 1")
case("r2_class_indented_end_after_backslash", f"@startuml\nclass Pine\nPine --> Larch : grows {B}\n   @enduml",
     ["class", "indented @enduml after backslash", "link label"], "link label 'grows    @enduml'")
case("r2_mindmap_indented_end_after_backslash", f"@startmindmap\n* Delta\n** Reed {B}\n  @endmindmap",
     ["mindmap", "indented @endmindmap after backslash"], "node 'Reed   @endmindmap'")
case("r2_seq_indented_end_then_more", f"@startuml\nWren -> Finch : one {B}\n  @enduml\nFinch -> Wren : two\n@enduml",
     ["sequence", "content after an indented end", "BlockUmlBuilder merged-line directive test"],
     "messages 'one   @enduml' (line 1) and 'two' (line 3)")
case("r2_seq_col0_end_swallowed_then_indented", f"@startuml\nWren -> Finch : one {B}\n@enduml\n  @enduml",
     ["sequence", "column-0 end swallowed", "Pipe stops after its end"], "No valid @start/@end found (exit 100)", valid=False)
case("r2_seq_named_start_indented_end", f"@startuml gull\nWren -> Finch : one {B}\n  @enduml",
     ["sequence", "named start: expectedEnd @end", "indented end"], "message 'one   @enduml' at line 1")
case("r2_seq_indented_start_and_end", f"  @startuml\nWren -> Finch : one {B}\n  @enduml",
     ["sequence", "indented start: Pipe START_MARK_NOT_FOUND", "indented end"],
     "message 'one   @enduml'; data-source-line 2 (Pipe prepends @startuml; Rust says 1: declared residue)")
case("r2_state_indented_end_after_comment", f"@startuml\n[*] --> Calm : rest {B}\n' aside\n  @enduml",
     ["state", "ReadFilterQuoteComment skip", "indented end"], "transition label 'rest   @enduml'")
case("r2_seq_indented_end_crlf", f"@startuml\nWren -> Finch : late {B}\n  @enduml",
     ["sequence", "indented end", "CRLF"], "message 'late   @enduml' at line 1", crlf=True)

# Obligation 2: UmlSource#loadInternal over Java's TIM result list.
case("r2_seq_include_tail_then_blank", "@startuml\n!include r2_tail.iuml\n\nEgret -> Heron : after\n@enduml",
     ["sequence", "!include tail", "blank TIM line"], "message 'drift ' at line 2, 'after' at line 3")
case("r2_seq_include_tail_then_comment", "@startuml\n!include r2_tail.iuml\n' aside\nEgret -> Heron : after\n@enduml",
     ["sequence", "!include tail", "' comment is no TIM line"], "message 'drift Egret -> Heron : after' at line 3")
case("r2_seq_include_tail_then_if", "@startuml\n!include r2_tail.iuml\n!$on = 1\n!if ($on == 1)\nEgret -> Heron : inside\n!endif\n@enduml",
     ["sequence", "!include tail", "!if block"], "message 'drift Egret -> Heron : inside' at line 4")
case("r2_seq_include_tail_then_include", "@startuml\n!include r2_tail.iuml\n!include r2_plain.iuml\nEgret -> Heron : main\n@enduml",
     ["sequence", "!include tail", "second include"],
     "message 'drift Egret -> Heron : plain' at r2_plain.iuml's location 0 (T14.53), 'main' at line 3")
case("r2_seq_include_two_tails_then_define", "@startuml\n!include r2_tails.iuml\n!define WHO Heron\nEgret -> WHO : main\n@enduml",
     ["sequence", "two included tails", "!define"], "message 'one and Egret -> Heron : main' at line 3")
case("r2_class_include_link_tail_before_end", "@startuml\nclass Alder\nclass Rowan\n!include r2_link.iuml\n@enduml",
     ["class", "!include tail before @enduml", "link label"], "link label 'tail @enduml' at line 4")
case("r2_seq_include_tail_no_end", "@startuml\nEgret -> Heron : head\n!include r2_tail.iuml",
     ["sequence", "!include tail", "no end: Pipe appends @enduml"], "messages 'head' (1), 'drift @enduml' (3)")
case("r2_component_include_tail_then_variable", "@startuml\n!include r2_comp.iuml\n!$n = 3\n[Rowan] --> [Ash] : n$n\n@enduml",
     ["component", "!include tail", "!$ assignment"],
     "link label 'wire [Rowan] --> [Ash] : n3' at line 3 (Rust routes the joined line to sequence: T14.21)")

# Obligation 3: ReadFilterMergeLines#isDitaa.
ART = f"+--------+\n| Ash {B}\n| Elm    |\n+--------+"
ART_JOINED = "+--------+\n| Ash | Elm    |\n+--------+"
case("r2_ditaa_continued_row", f"@startditaa\n{ART}\n@endditaa",
     ["ditaa", "@startditaa: join off"], "4 art rows; PNG differs from r2_ditaa_continued_row_joined")
case("r2_ditaa_continued_row_joined", f"@startditaa\n{ART_JOINED}\n@endditaa",
     ["ditaa", "control: continuation applied by hand"], "3 art rows")
case("r2_uml_ditaa_continued_row", f"@startuml\nditaa\n{ART}\n@enduml",
     ["ditaa", "@startuml + ditaa: join on"], "PNG byte-identical to r2_uml_ditaa_continued_row_joined")
case("r2_uml_ditaa_continued_row_joined", f"@startuml\nditaa\n{ART_JOINED}\n@enduml",
     ["ditaa", "control: continuation applied by hand"], "3 art rows")
case("r2_ditaa_backslash_before_end", f"@startditaa\n+-----+\n| Oak |\n+-----+ {B}\n@endditaa",
     ["ditaa", "continued last row before @endditaa"], "renders (the end is not swallowed)")

(out / "cases.json").write_text(json.dumps(cases, indent=1) + "\n")
print(len(cases), sum(c["intended_valid"] for c in cases), len(RESOURCES))
