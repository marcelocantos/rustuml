"""Generate held-out backslash-continuation probes (batch mc1) for a Java reference request.

Usage: python3 -I gen_probes.py OUT_DIR
Writes OUT_DIR/cases.json, OUT_DIR/<name>.puml and the include resources. No Java is run here.
Every case is outside the golden corpus; names, labels and shapes are fresh.
"""
import json, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=False)
B = "\\"
cases = []
def case(name, body, axes, valid=True, crlf=False):
    text = body if body.endswith("\n") else body + "\n"
    data = text.replace("\n", "\r\n") if crlf else text
    (out / f"{name}.puml").write_bytes(data.encode())
    cases.append(dict(name=name, intended_valid=valid, crlf=crlf, axes=axes))

RESOURCES = {
    "mc1_seq_inc.iuml": f"Kestrel -> Osprey : north {B}\nwind\n",
    "mc1_class_inc.iuml": f"class Lantern {B}\n<<lamp>>\nclass Wick\n",
    "mc1_proc_inc.iuml": f"!procedure $hail($who)\nHarbor -> $who : ahoy {B}\nmate\n!endprocedure\n",
}
for path, text in RESOURCES.items():
    (out / path).write_bytes(text.encode())

case("mc1_seq_procedure_body", f"@startuml\n!procedure $ping($to)\nDock -> $to : ping {B}\nback\n!endprocedure\n$ping(Pier)\n$ping(Quay)\nPier -> Quay : done\n@enduml",
     ["sequence", "!procedure body", "pre-TIM join", "following source line"])
case("mc1_seq_define_macro", f"@startuml\n!define SAY(a, b) a -> b : said {B}\n  softly\nSAY(Fern, Moss)\nMoss -> Fern : ok\n@enduml",
     ["sequence", "!define macro body", "leading whitespace kept"])
case("mc1_class_definelong", f"@startuml\n!definelong BOX(n)\nclass n {B}\n<<crate>>\nclass n ## Lid\n!enddefinelong\nBOX(Cedar)\nclass Birch\n@enduml",
     ["class", "!definelong body", "stereotype via continuation"])
case("mc1_seq_theme_plain", f"@startuml\n!theme plain\nGull -> Tern : over {B}\nthe {B}\nbay\nTern -> Gull : yes\n@enduml",
     ["sequence", "!theme", "three pieces"])
case("mc1_class_theme_sketchy", f"@startuml\n!theme sketchy-outline\nclass Anvil {B}\n<<iron>>\nclass Forge\nAnvil --> Forge : heat {B}\ned\n@enduml",
     ["class", "!theme", "link label"])
case("mc1_state_theme_bluegray", f"@startuml\n!theme bluegray\n[*] --> Thaw : warm {B}\ning\nThaw --> Melt\n@enduml",
     ["state", "!theme", "transition label"])
case("mc1_usecase_continued", f"@startuml\nactor Rowan\nRowan --> (Climb {B}\nridge)\n@enduml",
     ["usecase", "inside parentheses"])
case("mc1_seq_include_postjoin", "@startuml\n!include mc1_seq_inc.iuml\nOsprey -> Kestrel : calm\n@enduml",
     ["sequence", "!include file", "post-TIM UmlSource join"])
case("mc1_class_include_postjoin", "@startuml\n!include mc1_class_inc.iuml\nLantern --> Wick\n@enduml",
     ["class", "!include file", "post-TIM UmlSource join"])
case("mc1_seq_include_procedure", "@startuml\n!include mc1_proc_inc.iuml\n$hail(Skiff)\nSkiff -> Harbor : aye\n@enduml",
     ["sequence", "procedure from !include", "post-TIM join"])
case("mc1_class_three_pieces_lines", f"@startuml\nclass Quill {B}\n<<pen>> {B}\n#lightblue\nclass Ink\nclass Blotter\nQuill --> Ink\n@enduml",
     ["class", "three pieces", "following data-source-line"])
case("mc1_component_comments_between", f"@startuml\n[Valve] --> [Gauge] : press {B}\n' skipped line\n/' long\n still '/\n/' inline '/ ure\n[Gauge] --> [Dial]\n@enduml",
     ["component", "ReadFilterQuoteComment skip", "removeInnerComment on continuation"])
case("mc1_seq_note_body", f"@startuml\nMarten -> Vole : hop\nnote right of Vole\nfirst {B}\nsecond\nthird\nend note\nVole -> Marten : back\n@enduml",
     ["sequence", "multiline note body"])
case("mc1_activity_multiline_action", f"@startuml\nstart\n:Brew {B}\ntea\nand cake;\nstop\n@enduml",
     ["activity3", "multiline action body"])
case("mc1_state_description", f"@startuml\nstate Dormant : sleeps {B}\nsoundly\n[*] --> Dormant\nDormant --> Awake\n@enduml",
     ["state", "description"])
case("mc1_seq_escaped_backslash", "@startuml\nRaven -> Crow : path C:" + B + B + "\nCrow -> Raven : got it\n@enduml",
     ["sequence", "escaped double backslash not continued"])
case("mc1_seq_backslash_space", "@startuml\nRaven -> Crow : spaced " + B + " \nCrow -> Raven : next\n@enduml",
     ["sequence", "backslash then space not continued"])
case("mc1_seq_crlf", f"@startuml\nHeron -> Egret : wading {B}\nslowly\nEgret -> Heron : fine\n@enduml",
     ["sequence", "CRLF"], crlf=True)
case("mc1_class_crlf_three", f"@startuml\nclass Pebble {B}\n<<stone>> {B}\n#tan\nclass Sand\n@enduml",
     ["class", "CRLF", "three pieces"], crlf=True)
case("mc1_seq_before_enduml", f"@startuml\nLynx -> Ibex : last {B}\n@enduml",
     ["sequence", "backslash before @enduml (swallows the end)"], valid=False)
case("mc1_seq_at_eof", f"@startuml\nLynx -> Ibex : last {B}",
     ["sequence", "backslash at EOF without @enduml"], valid=False)
case("mc1_seq_foreach_body", f"@startuml\n!foreach $n in [Ash, Elm]\nRoot -> $n : grow {B}\ntall\n!endfor\nRoot -> Ash : end\n@enduml",
     ["sequence", "!foreach body"])
case("mc1_seq_tim_assignment", f"@startuml\n!$label = \"tide \" + {B}\n\"pool\"\nShore -> Reef : $label\n@enduml",
     ["sequence", "TIM assignment continued"])
case("mc1_seq_if_condition", f"@startuml\n!$on = 1\n!if $on == {B}\n1\nCove -> Bight : on\n!endif\nBight -> Cove : out\n@enduml",
     ["sequence", "!if condition continued"])
case("mc1_seq_comment_only_continuation", f"@startuml\nOtter -> Mink : swim {B}\n' quiet\n  ' again\nfast\nMink -> Otter : ok\n@enduml",
     ["sequence", "comment lines skipped before continuation"])
case("mc1_seq_continued_comment_line", f"@startuml\n' note {B}\nOtter -> Mink : hidden\nMink -> Otter : shown\n@enduml",
     ["sequence", "continued line is a comment: continuation joins the comment"])
case("mc1_mindmap", f"@startmindmap\n* Orchard {B}\nrows\n** Pear\n** Plum {B}\ntree\n@endmindmap",
     ["mindmap", "non-UML family pre-TIM join"])
case("mc1_json", "@startjson\n{\"crate\": " + B + "\n\"apples\", \"count\": 4}\n@endjson",
     ["json", "non-UML family pre-TIM join"])
case("mc1_skinparam_continued", f"@startuml\nskinparam class {B}\nBackgroundColor PaleGreen\nclass Fennel\nclass Dill\n@enduml",
     ["class", "skinparam line continued"])
case("mc1_style_block", f"@startuml\n<style>\nclassDiagram {B}\n{{\n  class {{ BackgroundColor Wheat }}\n}}\n</style>\nclass Barley\n@enduml",
     ["class", "<style> body continued"])
case("mc1_seq_leading_space_continuation", f"@startuml\nFinch -> Wren : a{B}\n   b\nWren -> Finch : c\n@enduml",
     ["sequence", "continuation leading spaces kept"])
case("mc1_object_continued", f"@startuml\nobject Kettle {B}\n<<pot>>\nobject Stove\nKettle --> Stove\n@enduml",
     ["object", "continued declaration"])
case("mc1_seq_block_comment_continued", f"@startuml\nPuffin -> Auk : wave {B}\n/' a\nb '/\n  hello\nAuk -> Puffin : hi\n@enduml",
     ["sequence", "long comment skipped before continuation"])

(out / "cases.json").write_text(json.dumps(cases, indent=1))
print(len(cases), sum(c["intended_valid"] for c in cases), len(RESOURCES))
