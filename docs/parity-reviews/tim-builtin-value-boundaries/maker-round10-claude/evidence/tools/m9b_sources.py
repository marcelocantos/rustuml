"""Round-9 maker relay batch M9B (obligation 2): a paragraph separator inside an otherwise all-RTL label carrying
an unmapped mark, in multi-line bodies (sequence note, activity action) whose lines no '.'-based command regex
matches, so Java keeps U+2029 / U+0085 in the label (BP showed single-line commands reject them).
With a PDF before the mark the second paragraph has no strong character: java.text.Bidi's direction is RTL with
getLevelAt 0 there, but TextLine's single run is level 1. Usage: python3 -I m9b_sources.py <batch_dir>"""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); (D / "sources").mkdir(parents=True, exist_ok=True)
M = "\U0001D167"; V = "skinparam defaultFontName Verdana"
cases = []
def add(name, body, axes):
    for suffix, text in [("", body), ("_ctrl", body.replace(M, ""))]:
        (D / "sources" / f"{name}{suffix}.puml").write_text(text, encoding="utf-8")
        cases.append({"name": f"{name}{suffix}", "intended_valid": True, "axes": axes + (["control"] if suffix else [])})
add("m9b_seqnote_p2029_pdf", f"@startuml\n{V}\nparticipant Ann\nnote over Ann\n\u05d0\u05d1\u2029\u202c {M}\nend note\n@enduml\n", ["textline-runs", "sequence", "note"])
add("m9b_seqnote_nel_pdf", f"@startuml\n{V}\nparticipant Ann\nnote over Ann\n\u05d2\u05d3\u0085\u202c {M}\nend note\n@enduml\n", ["textline-runs", "sequence", "note"])
add("m9b_act_p2029_pdf", f"@startuml\n{V}\nstart\n:\u05d4\u05d5\u2029\u202c {M}\nquay;\nstop\n@enduml\n", ["textline-runs", "activity"])
add("m9b_seqnote_p2029_mixed", f"@startuml\n{V}\nparticipant Ann\nnote over Ann\n\u05d0\u05d1\u2029{M}\nend note\n@enduml\n", ["textline-runs", "sequence", "note", "mixed"])
(D / "cases.json").write_text(json.dumps({"batch": "M9B", "cases": cases}, indent=1, ensure_ascii=False) + "\n")
print(len(cases), "cases")
