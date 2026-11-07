"""Checker round-5 (Opus) held-out probes, batch CR5O, for T14.22 round 5 (maker tip 36843c258, code c4b5a8436).
Written before any Java result for these inputs exists. Predictions come from the pinned Java source (71806a237):
  - PSystemBuilder#createPSystem (210-212): UmlSource joins ending backslashes only when findStartTypes(source[0])
    contains SEQUENCE, i.e. the start line is [@\\]startuml (DiagramType#getTypes 'u', regionMatches ignoreCase).
  - UmlSource#loadInternal (118-134): one pending StringBuilder over the whole TIM result list, in order.
  - ReadFilterMergeLines (Preprocessor, main source only; not for ditaa): raw main-source lines ending in '\\' are
    merged before TIM for every diagram type. Theme bodies (TContext#executeTheme) and raw includes
    (TContext#executeInclude, ReadLineReader) are not merged before TIM.
  - TContext#buildCodeIterator it30/it40: CodeIteratorShortComment drops TLineType.COMMENT_SIMPLE (^\\s*' or a
    whole-line /'...'/) with no diagram-type test; CodeIteratorInnerComment applies StringLocated#removeInnerComment.
  - StartUtils#beforeStartUml / isLetterOrDigit under JDK 21 (Unicode 15.0) for the include header test.
intended_valid is the checker's Java-validity prediction; 'unsure' is marked in axes.
Usage: python3 -I gen_probes_cr5o.py OUT_DIR"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
BS = "%chr(92)"  # a TIM-produced trailing backslash: the raw line does not end with '\\', so no reader merge.
def hdr(c): return f"{c} @startuml\n{c} Pukeko -> Takahe : hdr\n{c} @enduml\n"
RES = {
 # themes (read raw; joined post-TIM only under @startuml)
 "puml-theme-cr5obg.puml": "skinparam BackgroundColor \\\n#DDEEDD\n",
 "puml-theme-cr5oflat.puml": "skinparam BackgroundColor #DDEEDD\n",
 "puml-theme-cr5oend.puml": "skinparam ArrowColor #0000AA\nskinparam ArrowThickness \\\n",
 "puml-theme-cr5onest.puml": "!theme cr5oend from .\n3\nskinparam ParticipantBackgroundColor \\\n#FFDDAA\n",
 "puml-theme-cr5ohead.puml": "#AA00AA\nskinparam ParticipantBorderColor #00AAAA\n",
 "puml-theme-cr5otim.puml": f"skinparam ArrowColor {BS}\n#AA5500\n",
 "puml-theme-cr5ocmt.puml": "skinparam ArrowColor #993300 /' c '/\n/' d '/ skinparam ArrowThickness 2\n' short\n",
 "puml-theme-cr5otitle.puml": "title Ruru \\\nHoiho\n",
 # raw includes
 "cr5o_inc_cont.iuml": "Pukeko -> Takahe : inc \\\ncontinued\n",
 "cr5o_inc_mm.iuml": "* Matai \\\n** Miro\n",
 "cr5o_inc_open.iuml": "skinparam ArrowColor \\\n",
 "cr5o_inc_cmt.iuml": "Pukeko -> Takahe : inc /' c '/\n/' d '/ Takahe -> Pukeko : back\n",
 "cr5o_w_cont.iuml": "@startmindmap\n* Matai \\\nMiro\n@endmindmap\n",
 "cr5o_subs.iuml": "!startsub S\nPukeko -> Takahe : sub /' c '/\n!endsub\n",
 # include headers: Java 15.0 letter/digit => no header (raw include); otherwise header removed
 "cr5o_h_0c5c.iuml": hdr("\u0c5c"),   # Unicode 17 Lo; Cn in 15.0 -> header
 "cr5o_h_0cdc.iuml": hdr("\u0cdc"),   # Unicode 17 Lo; Cn in 15.0 -> header
 "cr5o_h_a7f1.iuml": hdr("\ua7f1"),   # Unicode 17 Lm; Cn in 15.0 -> header
 "cr5o_h_1c8a.iuml": hdr("\u1c8a"),   # Unicode 16 Ll; Cn in 15.0 -> header
 "cr5o_h_a7c0.iuml": hdr("\ua7c0"),   # Unicode 14 Lu -> letter -> raw
 "cr5o_h_0870.iuml": hdr("\u0870"),   # Unicode 14 Lo -> letter -> raw
 "cr5o_h_0cf3.iuml": hdr("\u0cf3"),   # Unicode 15 Mc -> not letter -> header
 "cr5o_h_02b0.iuml": hdr("\u02b0"),   # Lm -> raw
 "cr5o_h_0301.iuml": hdr("\u0301"),   # Mn -> header
 "cr5o_h_2160.iuml": hdr("\u2160"),   # Nl -> header
 "cr5o_h_00b2.iuml": hdr("\u00b2"),   # No -> header
 "cr5o_h_0966.iuml": hdr("\u0966"),   # Nd -> raw
 "cr5o_h_ac00.iuml": hdr("\uac00"),   # Lo (Hangul) -> raw
 "cr5o_h_1d400.iuml": hdr("\U0001d400"),  # supplementary Lu: two surrogate units, neither a letter -> header
 "cr5o_h_0378.iuml": hdr("\u0378"),   # Cn in 15 and 17 -> header
}
U = lambda body: "@startuml\n" + body + "@enduml\n"
C = []
def case(name, src, valid, axes): C.append(dict(name="cr5o_" + name, src=src, intended_valid=valid, axes=axes))
# 1. Diagram-type scope of the post-TIM join (themes, raw includes, TIM-produced backslashes)
case("scope_uml_usecase_theme", U("!theme cr5obg from .\nactor Hihi\n(Fly) as F\nHihi --> F\n"), True, ["scope", "@startuml usecase", "theme join"])
case("scope_uml_component_theme", U("!theme cr5obg from .\n[Kotare] --> [Kaka]\n"), True, ["scope", "@startuml component", "theme join"])
case("scope_uml_timing_theme", U("!theme cr5obg from .\nrobust \"Weka\" as W\n@0\nW is Idle\n"), True, ["scope", "@startuml timing", "theme join"])
case("scope_uml_upper_start", "@STARTUML\n!theme cr5obg from .\nPukeko -> Takahe : up\n@ENDUML\n", True, ["scope", "@STARTUML (findStartTypes ignores case)", "unsure"])
case("scope_uml_leading_ws_start", "  @startuml\n!theme cr5obg from .\nPukeko -> Takahe : ws\n@enduml\n", True, ["scope", "leading whitespace before @startuml"])
case("scope_uml_backslash_start", "\\startuml\n!theme cr5obg from .\nPukeko -> Takahe : bsl\n\\enduml\n", True, ["scope", "\\startuml", "unsure"])
case("scope_mindmap_flat_ctrl", "@startmindmap\n!theme cr5oflat from .\n* Matai\n** Miro\n@endmindmap\n", True, ["scope", "mindmap", "control"])
case("scope_mindmap_title_theme", "@startmindmap\n!theme cr5otitle from .\n* Matai\n** Miro\n@endmindmap\n", True, ["scope", "mindmap", "theme title not joined", "unsure"])
case("scope_gantt_theme", "@startgantt\n!theme cr5obg from .\n[Dig] lasts 3 days\n@endgantt\n", False, ["scope", "gantt", "no join: #DDEEDD line", "unsure"])
case("scope_json_theme", "@startjson\n!theme cr5obg from .\n{\"Ruru\": 1}\n@endjson\n", False, ["scope", "json", "no join", "unsure"])
case("scope_yaml_theme", "@startyaml\n!theme cr5obg from .\nRuru: 1\n@endyaml\n", False, ["scope", "yaml", "no join", "unsure"])
case("scope_ebnf_theme", "@startebnf\n!theme cr5obg from .\nruru = \"a\" ;\n@endebnf\n", False, ["scope", "ebnf", "no join", "unsure"])
case("scope_salt_theme", "@startsalt\n!theme cr5obg from .\n{\n[Matai]\n}\n@endsalt\n", False, ["scope", "salt", "no join", "unsure"])
case("scope_nwdiag_theme", "@startnwdiag\n!theme cr5obg from .\nnwdiag {\nnetwork lan {\nruru;\n}\n}\n@endnwdiag\n", False, ["scope", "nwdiag", "no join", "unsure"])
case("scope_wbs_flat_ctrl", "@startwbs\n!theme cr5oflat from .\n* Matai\n** Miro\n@endwbs\n", True, ["scope", "wbs", "control"])
case("scope_mindmap_raw_include", "@startmindmap\n!include cr5o_inc_mm.iuml\n@endmindmap\n", True, ["scope", "mindmap", "raw include not merged nor joined"])
case("scope_uml_raw_include", U("!include cr5o_inc_cont.iuml\n"), True, ["scope", "@startuml", "raw include joined post-TIM"])
case("scope_mindmap_wrapped_include", "@startmindmap\n!include cr5o_w_cont.iuml\n@endmindmap\n", True, ["scope", "mindmap", "wrapped include: DiagramDetector merges"])
case("scope_mindmap_tim_bs", f"@startmindmap\n* Matai {BS}\n** Miro\n@endmindmap\n", True, ["scope", "mindmap", "TIM-produced backslash, no join"])
case("scope_wbs_tim_bs", f"@startwbs\n* Matai {BS}\n** Miro\n@endwbs\n", True, ["scope", "wbs", "TIM-produced backslash, no join"])
case("scope_uml_tim_bs", U(f"Pukeko -> Takahe : tim {BS}\njoined\n"), True, ["scope", "@startuml", "TIM-produced backslash joined"])
case("scope_class_tim_bs", U(f"class Kaka {BS}\n{{\n+wing\n}}\n"), True, ["scope", "@startuml class", "TIM backslash joins a class header", "unsure"])
case("scope_gantt_tim_bs", f"@startgantt\n[Dig] lasts {BS}\n3 days\n@endgantt\n", False, ["scope", "gantt", "no join: '[Dig] lasts \\' alone", "unsure"])
# 2. Cross-stream join order (one pending over Java's result list)
case("join_main_into_theme", U(f"title Ruru {BS}\n!theme cr5oflat from .\nPukeko -> Takahe : m\n"), True, ["join", "main pending folds into the theme's first line"])
case("join_theme_open_then_end", U("Pukeko -> Takahe : e\n!theme cr5oend from .\n"), True, ["join", "theme open tail joins @enduml", "unsure"])
case("join_nested_open_into_outer", U("!theme cr5onest from .\nPukeko -> Takahe : n\n"), True, ["join", "nested open tail joins outer body line", "outer open joins main line"])
case("join_theme_tim_bs", U("!theme cr5otim from .\nPukeko -> Takahe : t\n"), True, ["join", "TIM-produced backslash inside a theme body"])
case("join_two_themes", U("!theme cr5oend from .\n!theme cr5ohead from .\nPukeko -> Takahe : two\n"), True, ["join", "theme A open tail joins theme B first line"])
case("join_include_open_into_theme", U("!include cr5o_inc_open.iuml\n!theme cr5ohead from .\nPukeko -> Takahe : i\n"), True, ["join", "raw include open tail joins theme first line"])
case("join_theme_open_then_directive", U("!theme cr5oend from .\n!$w = 4\n$w\nPukeko -> Takahe : d\n"), True, ["join", "a TIM directive is not a result line"])
case("join_proc_bs_line", U(f"!procedure P($a)\n$a -> Takahe : p {BS}\n!endprocedure\nP(Pukeko)\nafter\n"), True, ["join", "procedure output line ends with TIM backslash"])
# 3. Inner/short comment stripping
case("cmt_seq_string_inner", U("Pukeko -> Takahe : \"a /' b '/\" /' c '/\n"), True, ["comment", "trailing inner comment after a quoted /' '/"])
case("cmt_seq_nested", U("/' a /' b '/ Pukeko -> Takahe : nest\n"), True, ["comment", "leading: text after the first '/"])
case("cmt_seq_open_only", U("Pukeko -> Takahe : a /' b\nTakahe -> Pukeko : c '/\n"), True, ["comment", "no inner comment without both ends"])
case("cmt_seq_tab_lead", U("\t/' t '/ Pukeko -> Takahe : tab\n"), True, ["comment", "tab before a leading inner comment"])
case("cmt_special_only_line", U("/'''x'''/\nPukeko -> Takahe : s\n"), True, ["comment", "special-only line is a whole-line comment (TLineType COMMENT_SIMPLE2)"])
case("cmt_special_in_assign", U("!$y = \"Pu/'''z'''/keko\"\n$y -> Takahe : v\n"), True, ["comment", "special inner comment inside a quoted assignment"])
case("cmt_assign_trailing", U("!$x = \"Hihi\" /' c '/\n$x -> Takahe : a\n"), True, ["comment", "trailing inner comment on an assignment"])
case("cmt_if_trailing", U("!if 1 /' c '/\nPukeko -> Takahe : if\n!endif /' d '/\n"), True, ["comment", "inner comments on !if/!endif"])
case("cmt_procedure_body", U("!procedure Q($a)\n$a -> Takahe : one /' c '/\n/' d '/ $a -> Takahe : two\n!endprocedure\nQ(Pukeko)\n"), True, ["comment", "procedure body"])
case("cmt_function_return", U("!function F($x)\n!return $x + \"!\" /' c '/\n!endfunction\nPukeko -> Takahe : F(\"hi\")\n"), True, ["comment", "function !return"])
case("cmt_while_body", U("!$i = 0\n!while $i < 2\nPukeko -> Takahe : w$i /' c '/\n!$i = $i + 1\n!endwhile\n"), True, ["comment", "while body"])
case("cmt_foreach_body", U("!foreach $n in [\"Pukeko\", \"Ruru\"]\n/' c '/ $n -> Takahe : f\n!endfor\n"), True, ["comment", "foreach body"])
case("cmt_sub_body", U("!includesub cr5o_subs.iuml!S\n"), True, ["comment", "includesub body"])
case("cmt_include_body", U("!include cr5o_inc_cmt.iuml\n"), True, ["comment", "raw include body"])
case("cmt_theme_body", U("!theme cr5ocmt from .\nPukeko -> Takahe : th\n"), True, ["comment", "theme body: inner and short comments"])
case("cmt_ebnf_short_line", "@startebnf\nruru = \"a\" ,\n'b' ;\n@endebnf\n", False, ["comment", "ebnf", "^\\s*' is COMMENT_SIMPLE: the 'b' line is dropped", "unsure"])
case("cmt_ebnf_short_indented", "@startebnf\nruru = \"a\" | \"c\" ;\n  'note line\nhoiho = \"d\" ;\n@endebnf\n", True, ["comment", "ebnf", "indented short comment dropped"])
case("cmt_ebnf_trailing_multi", "@startebnf\nruru = \"a\" /' x '/ , \"b\" ;\nhoiho = \"c\" ; /' y '/\n@endebnf\n", True, ["comment", "ebnf", "trailing: text before the last /'"])
case("cmt_mindmap_inner", "@startmindmap\n* Matai /' c '/\n/' d '/ ** Miro\n@endmindmap\n", True, ["comment", "mindmap"])
case("cmt_wbs_inner", "@startwbs\n* Matai /' c '/\n/' d '/ ** Miro\n@endwbs\n", True, ["comment", "wbs"])
case("cmt_json_inner", "@startjson\n{\"Ruru\": 1} /' c '/\n@endjson\n", True, ["comment", "json"])
case("cmt_yaml_inner", "@startyaml\nRuru: 1 /' c '/\n/' d '/ Hoiho: 2\n@endyaml\n", True, ["comment", "yaml"])
case("cmt_yaml_quote_key", "@startyaml\n'Ruru': 1\nHoiho: 2\n@endyaml\n", True, ["comment", "yaml", "'quoted key line is COMMENT_SIMPLE: dropped"])
case("cmt_gantt_inner", "@startgantt\n[Dig] lasts 3 days /' c '/\n@endgantt\n", True, ["comment", "gantt"])
case("cmt_salt_inner", "@startsalt\n{\n[Matai] /' c '/\n}\n@endsalt\n", True, ["comment", "salt"])
case("cmt_class_inner", U("class Kaka { /' c '/\n/' d '/ +wing\n}\n"), True, ["comment", "class body"])
# 4. Include header letters (JDK 21 = Unicode 15.0)
for key, valid, ax in [("0c5c", True, "U+0C5C Unicode 17 Lo -> header"), ("0cdc", True, "U+0CDC Unicode 17 Lo -> header"),
                       ("a7f1", True, "U+A7F1 Unicode 17 Lm -> header"), ("1c8a", True, "U+1C8A Unicode 16 Ll -> header"),
                       ("a7c0", False, "U+A7C0 Unicode 14 Lu -> raw include"), ("0870", False, "U+0870 Unicode 14 Lo -> raw include"),
                       ("0cf3", True, "U+0CF3 Unicode 15 Mc -> header"), ("02b0", False, "U+02B0 Lm -> raw include"),
                       ("0301", True, "U+0301 Mn -> header"), ("2160", True, "U+2160 Nl -> header"), ("00b2", True, "U+00B2 No -> header"),
                       ("0966", False, "U+0966 Nd -> raw include"), ("ac00", False, "U+AC00 Lo -> raw include"),
                       ("1d400", True, "U+1D400 supplementary -> surrogates -> header"), ("0378", True, "U+0378 Cn -> header")]:
    case(f"hdr_{key}", U(f"!include cr5o_h_{key}.iuml\n"), valid, ["header", ax, "raw-include validity unsure"])
for p, body in RES.items(): (out / p).write_text(body, encoding="utf-8")
for c in C: (out / f"{c['name']}.puml").write_text(c["src"], encoding="utf-8")
(out / "cases.json").write_text(json.dumps([dict(name=c["name"], intended_valid=c["intended_valid"], axes=c["axes"], crlf=False) for c in C], indent=1, ensure_ascii=False))
(out / "resources.json").write_text(json.dumps(sorted(RES), indent=1))
print(len(C), "cases", len(RES), "resources")
