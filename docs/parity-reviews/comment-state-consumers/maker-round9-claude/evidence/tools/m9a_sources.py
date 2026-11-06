"""Round-9 maker relay batch M9A: sources and cases.json. Usage: python3 -I m9a_sources.py <batch_dir>
Axes: italic-face (obligation 1), textline-runs (obligation 2), link-font (obligation 3). Each probe has a
control without the measured character where the probe's outcome depends on it."""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); (D / "sources").mkdir(parents=True, exist_ok=True)
M = "\U0001D167"   # MUSICAL SYMBOL COMBINING TREMOLO-1: Mn, unmapped in every modelled face
CN = "skinparam defaultFontName Courier New"
cases = []
def add(name, body, axes, mark=True, ctrl=True):
    for suffix, text in [("", body)] + ([("_ctrl", body.replace(M, ""))] if ctrl and mark else []):
        (D / "sources" / f"{name}{suffix}.puml").write_text(text, encoding="utf-8")
        cases.append({"name": f"{name}{suffix}", "intended_valid": True, "axes": axes + (["control"] if suffix else [])})

# Obligation 1: italic / bold-italic Courier New in sequence, activity and note text (and class/usecase variants).
add("m9a_ci_seq_message", f"@startuml\n{CN}\nparticipant Ann\nparticipant Bo\nAnn -> Bo : <i>Ferry k{M}</i>\n@enduml\n", ["italic-face", "sequence"])
add("m9a_ci_seq_participant", f"@startuml\n{CN}\nparticipant \"<i>Jetty v{M}</i>\" as A\nA -> A : ok\n@enduml\n", ["italic-face", "sequence"])
add("m9a_ci_seq_note", f"@startuml\n{CN}\nparticipant Ann\nnote over Ann : <i>Buoy m{M}</i>\n@enduml\n", ["italic-face", "sequence", "note"])
add("m9a_ci_seq_bolditalic", f"@startuml\n{CN}\nparticipant Ann\nparticipant Bo\nAnn -> Bo : <b><i>Mooring t{M}</i></b>\n@enduml\n", ["italic-face", "sequence"])
add("m9a_ci_act_action", f"@startuml\n{CN}\nstart\n:<i>Slipway h{M}</i>;\nstop\n@enduml\n", ["italic-face", "activity"])
add("m9a_ci_act_note", f"@startuml\n{CN}\nstart\n:load;\nnote right: <i>Tide n{M}</i>\nstop\n@enduml\n", ["italic-face", "activity", "note"])
add("m9a_ci_act_bolditalic", f"@startuml\n{CN}\nstart\n:<b><i>Harbour p{M}</i></b>;\nstop\n@enduml\n", ["italic-face", "activity"])
add("m9a_ci_class_note", f"@startuml\n{CN}\nclass Hull\nnote \"<i>Keel s{M}</i>\" as N1\nHull .. N1\n@enduml\n", ["italic-face", "class", "note"])
add("m9a_ci_note_left", f"@startuml\n{CN}\nclass Mast\nnote left of Mast : <i>Sail j{M}</i>\n@enduml\n", ["italic-face", "class", "note"])
add("m9a_ci_interface", f"@startuml\n{CN}\ninterface \"Rudder u{M}\" as R\n@enduml\n", ["italic-face", "class"])
add("m9a_ci_abstract_member", f"@startuml\n{CN}\nclass Anchor {{\n  {{abstract}} drop{M}()\n}}\n@enduml\n", ["italic-face", "class"])
add("m9a_ci_abstract_bold", f"@startuml\n{CN}\nskinparam classFontStyle bold\nabstract class \"Bow c{M}\" as B\n@enduml\n", ["italic-face", "class", "bolditalic"])
add("m9a_cb_bold_class", f"@startuml\n{CN}\nskinparam classFontStyle bold\nclass \"Stern d{M}\" as S\n@enduml\n", ["italic-face", "class", "bold-blocklisted"])
add("m9a_ci_usecase_actor", f"@startuml\n{CN}\nactor \"<i>Pilot e{M}</i>\" as P\n@enduml\n", ["italic-face", "usecase"])
add("m9a_ci_state_desc", f"@startuml\n{CN}\nstate Docked\nDocked : <i>Cleat f{M}</i>\n@enduml\n", ["italic-face", "state"])
add("m9a_ci_act_note_font", f"@startuml\n{CN}\nskinparam noteFontName Courier New\nstart\n:load;\nnote right: <i>Tide r{M}</i>\nstop\n@enduml\n", ["italic-face", "activity", "note"])
add("m9a_ci_class_note_font", f"@startuml\n{CN}\nskinparam noteFontName Courier New\nclass Hull\nnote \"<i>Keel o{M}</i>\" as N1\nHull .. N1\n@enduml\n", ["italic-face", "class", "note"])
# Other families' italic faces (facts differ; outcome predicted unchanged for these strings).
add("m9a_vi_hebrew_italic", f"@startuml\nskinparam defaultFontName Verdana\nclass K\nK : <i>Quay א {M}ב</i>\n@enduml\n", ["italic-face", "verdana"])
add("m9a_mi_menlo_italic", f"@startuml\nclass K\nK : <font:Monospaced><i>Quay g{M}</i></font>\n@enduml\n", ["italic-face", "menlo"])
add("m9a_ai_arial_italic", f"@startuml\nskinparam defaultFontName Arial\nclass K\nK : <i>Quay ب {M}</i>\n@enduml\n", ["italic-face", "arial"])

# Obligation 2: a paragraph separator inside an otherwise all-RTL label carrying an unmapped mark. With an
# (unmatched) PDF before the mark the second paragraph has no strong character: java.text.Bidi's direction is
# RTL with getLevelAt 0 there, but TextLine's run (BidiLine.getRuns) is one run at level 1.
for sep, tag in [("\u2029", "p2029"), ("\u0085", "nel")]:
    add(f"m9a_bidi_{tag}_pdf_verdana", f"@startuml\nskinparam defaultFontName Verdana\nclass \"\u05d0\u05d1{sep}\u202c{M}\" as K\n@enduml\n", ["textline-runs", "verdana"])
    add(f"m9a_bidi_{tag}_pdf_space_verdana", f"@startuml\nskinparam defaultFontName Verdana\nclass \"\u05d2\u05d3{sep}\u202c {M}\" as K\n@enduml\n", ["textline-runs", "verdana"])
# Same separators without the PDF: a MIXED paragraph, unchanged by obligation 2.
add("m9a_bidi_p2029_mixed_verdana", f"@startuml\nskinparam defaultFontName Verdana\nclass \"\u05d4\u05d5\u2029{M}\" as K\n@enduml\n", ["textline-runs", "verdana", "mixed"])

# Obligation 3: renamed held-out link-label variants (other Indic scripts, other links, other arrow fonts).
add("m9a_lt_verdana_beng_link", f"@startuml\nskinparam defaultFontName Verdana\nclass Hold\nclass Deck\nHold --> Deck : অ{M} y\n@enduml\n", ["link-font", "verdana"])
add("m9a_lt_arial_guru_link", f"@startuml\nskinparam defaultFontName Arial\nclass Port\nclass Berth\nPort ..> Berth : ਕ{M} q\n@enduml\n", ["link-font", "arial"])
add("m9a_lt_verdana_size_link", f"@startuml\nskinparam defaultFontName Verdana\nskinparam ArrowFontSize 17\nclass Quay\nclass Pier\nQuay -- Pier : க{M} z\n@enduml\n", ["link-font", "verdana", "arrow-font-size"])
add("m9a_lt_courier_telu_link", f"@startuml\n{CN}\nclass Keel\nclass Deck\nKeel <|-- Deck : క{M} w\n@enduml\n", ["link-font", "courier"])
add("m9a_lt_arrowfont_link", "@startuml\nskinparam ArrowFontName Courier New\nclass Ferry\nclass Route\nFerry --> Route : harbour shuttle\n@enduml\n", ["link-font", "arrow-font-name"], mark=False)
add("m9a_lt_arrowsize_link", "@startuml\nskinparam ArrowFontSize 19\nclass Ferry\nclass Route\nFerry --> Route : harbour shuttle\n@enduml\n", ["link-font", "arrow-font-size"], mark=False)
(D / "cases.json").write_text(json.dumps({"batch": "M9A", "cases": cases}, indent=1, ensure_ascii=False) + "\n")
print(len(cases), "cases")
