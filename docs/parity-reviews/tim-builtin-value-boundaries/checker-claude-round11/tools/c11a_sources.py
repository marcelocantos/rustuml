"""Checker round-11 (claude) held-out batch C11A. Usage: python3 -I c11a_sources.py <batch_dir>

All cases intended valid. Text uses plain Latin, digits and punctuation that hit every glyph where Verdana-Italic differs
from Verdana (A D V W g k m v y) and Verdana-BoldItalic from Verdana-Bold (o v); fresh words, not the maker's.
Groups:
 fc  face x consumer: Verdana and Arial in plain / italic / bold / bold-italic over 30 text consumers
     (bold-italic via a <style> FontStyle italic + FontWeight bold selector, or bold skinparam + <i> wrap where no
     selector is certain; recorded in axes).
 sz  sizes 9 and 23 (Verdana p/i/bi, Arial i) over 6 consumers.
 cr  mixed creole runs: <i>, <b>, <b><i>, **//..//**, bold label + <i>, italic label + <b> (Verdana 8 consumers, Arial 3).
 of  other families must not change: Times New Roman, Courier New, Helvetica, SansSerif, Monospaced, Lucida Grande,
     p/i/b/bi on component and participant.
 st  stereotypes (drawn italic) on Verdana / Arial elements with p/i/b element style.
 sx  StateNodeFont reachability (C5): every stateFontStyle spelling/form, Times/default/Courier/Verdana.
 qu  queue pill (C3): queue participants with stereotypes, multi-line, bold/italic, Verdana/Times.
 na  non-ASCII next to ASCII in Verdana italic / bold-italic (T14.55 residue; no claim).
"""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); S = D / "sources"; S.mkdir(parents=True, exist_ok=True)
FAM = {"vd": "Verdana", "ar": "Arial", "tn": "Times New Roman", "cn": "Courier New", "hv": "Helvetica", "ss": None,
       "mo": "Monospaced", "lg": "Lucida Grande"}
cases = []
def add(name, body, axes, fam=None, pre=""):
    head = "@startuml\n" + (f"skinparam defaultFontName {FAM[fam]}\n" if fam and FAM.get(fam) else "") + pre
    (S / f"{name}.puml").write_text(head + body.strip("\n") + "\n@enduml\n", encoding="utf-8")
    assert name not in {c["name"] for c in cases}, name
    cases.append({"name": name, "intended_valid": True, "axes": axes})

T = "Avid Dawky Vow 4.7, gym kayak ovo!"
# consumer: (body template with {T}, skinparam prefix, style selector or None)
CON = {
 "comp": ('component "{T}" as E1\n', "component", "component"),
 "node": ('node "{T}" as E1\ncomponent "Ok" as E2\nE1 --> E2\n', "node", "node"),
 "dbse": ('database "{T}" as E1\ncomponent "Ok" as E2\nE1 --> E2\n', "database", "database"),
 "clou": ('cloud "{T}" as E1\ncomponent "Ok" as E2\nE1 --> E2\n', "cloud", "cloud"),
 "rect": ('rectangle "{T}" as E1\n', "rectangle", "rectangle"),
 "pack": ('package "{T}" {{\n  component "Ok" as E1\n}}\n', "package", "package"),
 "clss": ('class "{T}" as E1\n', "class", "class"),
 "catt": ('class E1 {{\n  +{T}\n}}\n', "classAttribute", None),
 "objc": ('object "{T}" as E1\n', "object", "object"),
 "part": ('participant "{T}" as E1\nE1 -> E1 : ok\n', "participant", "participant"),
 "smsg": ('participant A\nparticipant B\nA -> B : {T}\n', "arrow", "arrow"),
 "snot": ('participant A\nnote right of A : {T}\n', "note", "note"),
 "sgrp": ('participant A\ngroup {T}\nA -> A : ok\nend\n', "sequenceGroupHeader", None),
 "sdiv": ('participant A\nA -> A : ok\n== {T} ==\nA -> A : ok\n', "sequenceDivider", None),
 "actv": ('start\n:{T};\nstop\n', "activity", "activity"),
 "diam": ('start\nif ({T}) then (yes)\n  :Ok;\nendif\n', "activityDiamond", None),
 "swim": ('|{T}|\nstart\n:Ok;\n', "swimlaneTitle", None),
 "anot": ('start\n:Ok;\nnote right : {T}\n', "note", "note"),
 "stat": ('state "{T}" as E1\n', "state", "state"),
 "sdsc": ('state E1\nE1 : {T}\n', "stateAttribute", None),
 "usec": ('usecase "{T}" as E1\n', "usecase", "usecase"),
 "actu": ('actor "{T}" as E1\nusecase "Ok" as E2\nE1 --> E2\n', "actor", "actor"),
 "clnk": ('component "A" as E1\ncomponent "B" as E2\nE1 --> E2 : {T}\n', "arrow", "arrow"),
 "kcls": ('class A\nclass B\nA --> B : {T}\n', "arrow", "arrow"),
 "ulnk": ('actor "A" as E1\nusecase "B" as E2\nE1 --> E2 : {T}\n', "arrow", "arrow"),
 "tlnk": ('state A\nstate B\nA --> B : {T}\n', "arrow", "arrow"),
 "titl": ('title {T}\ncomponent "Ok" as E1\n', "title", "title"),
 "lgnd": ('component "Ok" as E1\nlegend\n{T}\nendlegend\n', "legend", "legend"),
 "cnot": ('component "Ok" as E1\nnote right of E1 : {T}\n', "note", "note"),
 "knot": ('class E1\nnote right of E1 : {T}\n', "note", "note"),
}
def style_pre(kind, sel, st):
    if st == "p": return ""
    if st in ("i", "b"): return f"skinparam {kind}FontStyle {'italic' if st == 'i' else 'bold'}\n"
    if sel: return f"<style>\n{sel} {{\n  FontStyle italic\n  FontWeight bold\n}}\n</style>\n"
    return f"skinparam {kind}FontStyle bold\n"
for k, (body, kind, sel) in CON.items():
    for f in ["vd", "ar"]:
        for st in ["p", "i", "b", "bi"]:
            text = T if st != "bi" or sel else f"<i>{T}</i>"
            how = {"p": "plain", "i": "skinparam italic", "b": "skinparam bold",
                   "bi": "style italic+bold" if sel else "skinparam bold + <i> wrap"}[st]
            add(f"c11a_fc_{k}_{f}_{st}", body.format(T=text), ["face-consumer", k, f, st, how] + (["control"] if st == "p" else []),
                f, style_pre(kind, sel, st))

SZ = ["comp", "part", "clss", "actv", "stat", "usec"]
for k in SZ:
    body, kind, sel = CON[k]
    for size in [9, 23]:
        for f, st in [("vd", "p"), ("vd", "i"), ("vd", "bi"), ("ar", "i")]:
            add(f"c11a_sz{size}_{k}_{f}_{st}", body.format(T=T), ["size", k, str(size), f, st] + (["control"] if st == "p" else []),
                f, f"skinparam {kind}FontSize {size}\n" + style_pre(kind, sel, st))

CR = {"ir": ("p", "Wavy <i>Dog kim Vay</i> 9 ovo"), "br": ("p", "Wavy <b>Dog kim Vay</b> 9 ovo"),
      "bir": ("p", "Wavy <b><i>Dog kim Vay</i></b> 9 ovo"), "md": ("p", "Wavy **//Dog kim Vay//** 9 ovo"),
      "bl_i": ("b", "Wavy <i>Dog kim Vay</i> 9 ovo"), "il_b": ("i", "Wavy <b>Dog kim Vay</b> 9 ovo"),
      "ctl": ("p", "Wavy Dog kim Vay 9 ovo")}
CRC = ["comp", "part", "clss", "actv", "usec", "stat", "smsg", "cnot"]
for k in CRC:
    body, kind, sel = CON[k]
    for c, (st, text) in CR.items():
        for f in (["vd", "ar"] if k in ("comp", "part", "actv") else ["vd"]):
            add(f"c11a_cr_{k}_{c}_{f}", body.format(T=text), ["creole", k, c, f, st] + (["control"] if c == "ctl" else []),
                f, style_pre(kind, sel, st) if st != "bi" else "")

for f in ["tn", "cn", "hv", "ss", "mo", "lg"]:
    for k in ["comp", "part"]:
        body, kind, sel = CON[k]
        for st in ["p", "i", "b", "bi"]:
            add(f"c11a_of_{k}_{f}_{st}", body.format(T=T), ["other-family", k, f, st, "must not change"], f, style_pre(kind, sel, st))

STC = {"comp": ('component "Gym Dawk" as E1 <<Vivid kayak Wovo>>\n', "component"),
       "clss": ('class "Gym Dawk" as E1 <<Vivid kayak Wovo>>\n', "class"),
       "part": ('participant "Gym Dawk" as E1 <<Vivid kayak Wovo>>\nE1 -> E1 : ok\n', "participant"),
       "usec": ('usecase "Gym Dawk" as E1 <<Vivid kayak Wovo>>\n', "usecase")}
for k, (body, kind) in STC.items():
    for f in ["vd", "ar"]:
        for st in ["p", "i", "b"]:
            add(f"c11a_st_{k}_{f}_{st}", body, ["stereotype", k, f, st], f, style_pre(kind, None, st))

SXB = 'state "Kiln wq vista" as E1\nE1 : first desc line\nE1 : a longer second desc\nE1 --> E2\nstate "Second Dawky" as E2\n'
SX = {
 "sp": "skinparam stateFontStyle italic\n",
 "spcap": "skinparam StateFontStyle Italic\n",
 "spblk": "skinparam state {\n  FontStyle italic\n}\n",
 "spblkcap": "skinparam State {\n  fontstyle ITALIC\n}\n",
 "spster": "skinparam stateFontStyle<<Dock>> italic\n",
 "spbi": "skinparam stateFontStyle italic bold\n",
 "spobl": "skinparam stateFontStyle oblique\n",
 "spattr": "skinparam stateAttributeFontStyle italic\n",
 "sptwice": "skinparam stateFontStyle bold\nskinparam stateFontStyle italic\n",
 "spmono": "skinparam stateFontName Courier New\nskinparam stateFontStyle italic\n",
 "spsize": "skinparam stateFontSize 17\nskinparam stateFontStyle italic\n",
}
for k, pre in SX.items():
    for f in ["tn", "ss", "vd"]:
        add(f"c11a_sx_{k}_{f}", SXB, ["state-italic-reachability", k, f], f, pre)
for f in ["tn", "ss", "vd"]:
    add(f"c11a_sx_ctl_{f}", SXB, ["state-italic-reachability", "none", f, "control"], f)

QU = {"plain": 'queue "Dawky Vow" as Q\nparticipant A\nA -> Q : go\n',
      "stereo": 'queue "Dawky Vow" as Q <<Vivid kayak>>\nparticipant A\nA -> Q : go\n',
      "multi": 'queue "Dawky Vow\\nsecond gym line" as Q\nparticipant A\nA -> Q : go\n',
      "multist": 'queue "Dawky Vow\\nsecond gym line" as Q <<Vivid>>\nparticipant A\nA -> Q : go\n',
      "empty": 'queue "Dawky Vow\\n\\nthird" as Q\nparticipant A\nA -> Q : go\n'}
for k, body in QU.items():
    for f in ["vd", "tn"]:
        for st, pre in [("p", ""), ("i", "skinparam participantFontStyle italic\n"), ("b", "skinparam participantFontStyle bold\n")]:
            add(f"c11a_qu_{k}_{f}_{st}", body, ["queue-pill", k, f, st], f, pre)

NA = {"guil": 'component "Dawky «Vow» gym" as E1\n', "acc": 'component "Dåwky Vöw gym é" as E1\n',
      "heb": 'component "Dawky אב gym" as E1\n', "nst": 'component "Dawky gym" as E1 <<Vöw kayak>>\n'}
for k, body in NA.items():
    for st in ["p", "i", "bi"]:
        add(f"c11a_na_{k}_vd_{st}", body, ["non-ascii", k, "vd", st, "T14.55 residue, no claim"], "vd", style_pre("component", "component", st))

json.dump({"batch": "C11A", "cases": cases}, open(D / "cases.json", "w"), indent=1, ensure_ascii=False)
print(len(cases), "cases")
