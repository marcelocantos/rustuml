"""Checker round-10 (claude) held-out batch C10A. Usage: python3 -I c10a_sources.py <batch_dir>

Groups (all intended valid; every styled probe has a same-source unstyled twin where meaningful):
 st  stereotype face: element stereotypes (now measured italic by the packet) in Arial/Times/Verdana/Courier New/SansSerif,
     no FontStyle, over component, node/database/cloud/queue in component diagrams, usecase, actor (usecase diagram),
     sequence participant, class (out-of-packet control); plus Times italic label + stereotype.
 cl  cloud outline in component diagrams (emit_cloud_component now uses the element's family/size/style).
 ml  multi-line labels in italic/bold Times and Verdana with plain controls.
 cr  Creole inside italic Times labels (bold run, size, font, ** markup) with plain controls.
 pr  <style> versus skinparam precedence and selector forms (Times).
 ns  nested containers (package/node/folder, sequence box, composite state, rectangle, partition/swimlane).
 fv  FontStyle value forms (ValueImpl.asFontFace): case, CSS keywords/weights, two-word values, unknown words.
 bi  'bold italic' in consumers outside the packet (class, notes, arrows, title, legend, ...) with 'bold' controls.
 ft  face tables: full printable ASCII in each family's four faces (Arial, Verdana, Helvetica, Times, Courier New).
 sz  font size plus italic.
 sx  state specifics; ax activity specifics.
"""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); S = D / "sources"; S.mkdir(parents=True, exist_ok=True)
FAM = {"ar": "Arial", "tn": "Times New Roman", "vd": "Verdana", "cn": "Courier New", "ss": None, "hv": "Helvetica"}
cases = []
def add(name, body, axes, fam=None, pre=""):
    head = "@startuml\n" + (f"skinparam defaultFontName {FAM[fam]}\n" if fam and FAM.get(fam) else "") + pre
    (S / f"{name}.puml").write_text(head + body.strip("\n") + "\n@enduml\n", encoding="utf-8")
    cases.append({"name": name, "intended_valid": True, "axes": axes})

ST = "Ferrous gauge vista"
stctx = {
 "comp": f'component "Kiln" as E1 <<{ST}>>\n',
 "nodc": f'node "Kiln" as E1 <<{ST}>>\ncomponent "Ok" as E2\nE1 --> E2\n',
 "dbsc": f'database "Kiln" as E1 <<{ST}>>\ncomponent "Ok" as E2\nE1 --> E2\n',
 "cldc": f'cloud "Kiln" as E1 <<{ST}>>\ncomponent "Ok" as E2\nE1 --> E2\n',
 "quec": f'queue "Kiln" as E1 <<{ST}>>\ncomponent "Ok" as E2\nE1 --> E2\n',
 "usec": f'usecase "Kiln" as E1 <<{ST}>>\n',
 "actu": f'actor "Kiln" as E1 <<{ST}>>\nusecase "Ok" as E2\nE1 --> E2\n',
 "part": f'participant "Kiln" as E1 <<{ST}>>\nE1 -> E1 : ok\n',
 "clss": f'class "Kiln" as E1 <<{ST}>>\n',
}
for k, body in stctx.items():
    for f in ["ar", "tn", "vd", "cn", "ss"]:
        add(f"c10a_st_{k}_{f}", body, ["stereotype", k, f], f)
    kind = {"comp": "component", "nodc": "node", "dbsc": "database", "cldc": "cloud", "quec": "queue", "usec": "usecase",
            "actu": "actor", "part": "participant", "clss": "class"}[k]
    add(f"c10a_st_{k}_tn_i", body, ["stereotype", k, "tn", "label italic"], "tn", f"skinparam {kind}FontStyle italic\n")

# cl: cloud outline in component diagrams
for f in ["ss", "tn", "vd"]:
    for sz in [None, 20]:
        for stq in [False, True]:
            pre = f"skinparam cloudFontSize {sz}\n" if sz else ""
            st = f" <<{ST}>>" if stq else ""
            add(f"c10a_cl_{f}_{sz or 'd'}_{'s' if stq else 'n'}", f'cloud "Harbour wq vista" as E1{st}\ncomponent "Ok" as E2\nE1 --> E2\n',
                ["cloud", f, f"size {sz or 'default'}", "stereo" if stq else "no stereo"], f, pre)
    add(f"c10a_cl_{f}_i", 'cloud "Harbour wq vista" as E1\ncomponent "Ok" as E2\nE1 --> E2\n', ["cloud", f, "italic"], f, "skinparam cloudFontStyle italic\n")
add("c10a_cl_lone_tn", 'cloud "Harbour wq vista" as E1\n', ["cloud", "tn", "lone (deployment path)"], "tn")

# ml: multi-line labels
ML = "Kiln wq\\nlonger second line"
mlctx = {
 "comp": ("component", f'component "{ML}" as E1\n'),
 "nodc": ("node", f'node "{ML}" as E1\ncomponent "Ok" as E2\nE1 --> E2\n'),
 "part": ("participant", f'participant "{ML}" as E1\nE1 -> E1 : ok\n'),
 "actv": ("activity", 'start\n:Kiln wq\nlonger second line;\n'),
 "stat": ("state", 'state "Kiln wq" as E1\nE1 : first desc\nE1 : longer second description\n'),
 "usec": ("usecase", f'usecase "{ML}" as E1\n'),
 "actu": ("actor", f'actor "{ML}" as E1\nusecase "Ok" as E2\nE1 --> E2\n'),
}
for k, (kind, body) in mlctx.items():
    for f in ["tn", "vd"]:
        add(f"c10a_ml_{k}_{f}_p", body, ["multiline", k, f, "plain", "control"], f)
        add(f"c10a_ml_{k}_{f}_i", body, ["multiline", k, f, "italic"], f, f"skinparam {kind}FontStyle italic\n")
        add(f"c10a_ml_{k}_{f}_b", body, ["multiline", k, f, "bold"], f, f"skinparam {kind}FontStyle bold\n")

# cr: creole inside italic labels (Times)
CR = {"bt": "<b>Kiln</b> wq vista", "sz": "<size:20>Kiln</size> wq vista", "fo": "<font:Courier New>Kiln</font> wq vista", "st": "**Kiln** wq vista"}
crctx = {
 "comp": ("component", lambda t: f'component "{t}" as E1\n'),
 "part": ("participant", lambda t: f'participant "{t}" as E1\nE1 -> E1 : ok\n'),
 "actv": ("activity", lambda t: f'start\n:{t};\n'),
 "usec": ("usecase", lambda t: f'usecase "{t}" as E1\n'),
 "stat": ("state", lambda t: f'state "{t}" as E1\n'),
}
for k, (kind, mk) in crctx.items():
    for c, t in CR.items():
        add(f"c10a_cr_{k}_{c}_p", mk(t), ["creole", k, c, "plain", "control"], "tn")
        add(f"c10a_cr_{k}_{c}_i", mk(t), ["creole", k, c, "italic"], "tn", f"skinparam {kind}FontStyle italic\n")

# pr: <style> vs skinparam precedence and selectors (Times)
prctx = {
 "comp": ("component", 'component "Kiln wq vista" as E1 <<Dock>>\n'),
 "part": ("participant", 'participant "Kiln wq vista" as E1 <<Dock>>\nE1 -> E1 : ok\n'),
 "actv": ("activity", 'start\n:Kiln wq vista;\n'),  # no stereotype syntax on actions
 "stat": ("state", 'state "Kiln wq vista" as E1 <<Dock>>\n'),
 "usec": ("usecase", 'usecase "Kiln wq vista" as E1 <<Dock>>\n'),
}
DIAG = {"comp": "componentDiagram", "part": "sequenceDiagram", "actv": "activityDiagram", "stat": "stateDiagram", "usec": "usecaseDiagram"}
SEL = {"comp": "component", "part": "participant", "actv": "activity", "stat": "state", "usec": "usecase"}
for k, (kind, body) in prctx.items():
    sel = SEL[k]
    add(f"c10a_pr_{k}_sp_then_css", body, ["precedence", k, "skinparam italic then style bold"], "tn",
        f"skinparam {kind}FontStyle italic\n<style>\n{sel} {{\n  FontStyle bold\n}}\n</style>\n")
    add(f"c10a_pr_{k}_css_then_sp", body, ["precedence", k, "style bold then skinparam italic"], "tn",
        f"<style>\n{sel} {{\n  FontStyle bold\n}}\n</style>\nskinparam {kind}FontStyle italic\n")
    add(f"c10a_pr_{k}_nested_sel", body, ["precedence", k, "diagram-scoped selector italic"], "tn",
        f"<style>\n{DIAG[k]} {{\n  {sel} {{\n    FontStyle italic\n  }}\n}}\n</style>\n")
    if k != "actv": add(f"c10a_pr_{k}_stereo_sel", body, ["precedence", k, "stereotype selector italic"], "tn",
        f"<style>\n.Dock {{\n  FontStyle italic\n}}\n</style>\n")
    add(f"c10a_pr_{k}_element_sel", body, ["precedence", k, "element selector italic"], "tn",
        f"<style>\nelement {{\n  FontStyle italic\n}}\n</style>\n")
    add(f"c10a_pr_{k}_ctrl", body, ["precedence", k, "no style", "control"], "tn")

# ns: nested containers
NS = {
 "pkg": ("component", 'package "Dock" {\n  component "Kiln wq vista" as E1\n}\n'),
 "innode": ("component", 'node "Dock" {\n  component "Kiln wq vista" as E1\n}\n'),
 "folder": ("component", 'folder "Dock" {\n  component "Kiln wq vista" as E1\n  component "Ok" as E2\n}\nE1 --> E2\n'),
 "nodenode": ("node", 'node "Dock" {\n  node "Kiln wq vista" as E1\n}\ncomponent "Ok" as E2\nE1 --> E2\n'),
 "seqbox": ("participant", 'box "Harbour"\nparticipant "Kiln wq vista" as E1\nparticipant "Ok" as E2\nend box\nE1 -> E2 : go\n'),
 "compst": ("state", 'state Outer {\n  state "Kiln wq vista" as E1\n}\n'),
 "ucrect": ("usecase", 'rectangle Yard {\n  usecase "Kiln wq vista" as E1\n}\n'),
 "partition": ("activity", 'start\npartition Dock {\n  :Kiln wq vista;\n}\n'),
 "swim": ("activity", '|Dock|\nstart\n:Kiln wq vista;\n|Yard|\n:Ok;\n'),
}
for k, (kind, body) in NS.items():
    add(f"c10a_ns_{k}_tn_p", body, ["nested", k, "tn", "plain", "control"], "tn")
    add(f"c10a_ns_{k}_tn_i", body, ["nested", k, "tn", "italic"], "tn", f"skinparam {kind}FontStyle italic\n")
    add(f"c10a_ns_{k}_vd_b", body, ["nested", k, "vd", "bold"], "vd", f"skinparam {kind}FontStyle bold\n")

# fv: FontStyle value forms (asFontFace)
FV = {"BOLD": "BOLD", "Italic": "Italic", "bolder": "bolder", "lighter": "lighter", "w600": "600", "w650": "650", "w900": "900",
      "normal": "normal", "italicbold": "italic bold", "oblique": "oblique"}
fvctx = {
 "comp": ("component", 'component "Kiln wq vista" as E1\n'),
 "part": ("participant", 'participant "Kiln wq vista" as E1\nE1 -> E1 : ok\n'),
 "actv": ("activity", 'start\n:Kiln wq vista;\n'),
 "stat": ("state", 'state "Kiln wq vista" as E1\n'),
 "usec": ("usecase", 'usecase "Kiln wq vista" as E1\n'),
}
for k, (kind, body) in fvctx.items():
    for v, val in FV.items():
        add(f"c10a_fv_{k}_{v}", body, ["fontstyle-value", k, val], "tn", f"skinparam {kind}FontStyle {val}\n")
    add(f"c10a_fv_{k}_css800", body, ["fontstyle-value", k, "style 800"], "tn", f"<style>\n{SEL[k]} {{\n  FontStyle 800\n}}\n</style>\n")
    add(f"c10a_fv_{k}_cssbi", body, ["fontstyle-value", k, "style bold italic"], "tn", f"<style>\n{SEL[k]} {{\n  FontStyle bold italic\n}}\n</style>\n")
    add(f"c10a_fv_{k}_ctrl", body, ["fontstyle-value", k, "none", "control"], "tn")

# bi: 'bold italic' in consumers outside the packet
BI = {
 "classname": ("classFontStyle", 'class "Kiln wq vista" as E1\n'),
 "classattr": ("classAttributeFontStyle", 'class E1 {\n  +harbour : int\n}\n'),
 "compnote": ("noteFontStyle", 'component "Ok" as E1\nnote right of E1 : Kiln wq vista\n'),
 "seqnote": ("noteFontStyle", 'participant A\nnote right of A : Kiln wq vista\n'),
 "seqarrow": ("sequenceArrowFontStyle", 'participant A\nparticipant B\nA -> B : Kiln wq vista\n'),
 "comparrow": ("arrowFontStyle", 'component "A" as E1\ncomponent "B" as E2\nE1 --> E2 : Kiln wq vista\n'),
 "title": ("titleFontStyle", 'title Kiln wq vista\ncomponent "Ok" as E1\n'),
 "legend": ("legendFontStyle", 'component "Ok" as E1\nlegend\nKiln wq vista\nendlegend\n'),
 "pkg": ("packageFontStyle", 'package "Kiln wq vista" {\n  component "Ok" as E1\n}\n'),
 "seqbox": ("sequenceBoxFontStyle", 'box "Kiln wq vista"\nparticipant A\nend box\n'),
 "swim": ("swimlaneTitleFontStyle", '|Kiln wq vista|\nstart\n:Ok;\n'),
 "diamond": ("activityDiamondFontStyle", 'start\nif (Kiln wq vista?) then (yes)\n  :Ok;\nendif\n'),
 "statattr": ("stateAttributeFontStyle", 'state E1\nE1 : Kiln wq vista\n'),
 "objname": ("objectFontStyle", 'object "Kiln wq vista" as E1\n'),
}
for k, (sp, body) in BI.items():
    add(f"c10a_bi_{k}_bi", body, ["bold-italic-consumer", k, sp, "bold italic"], "tn", f"skinparam {sp} bold italic\n")
    add(f"c10a_bi_{k}_p", body, ["bold-italic-consumer", k, sp, "none", "control"], "tn")

# ft: face tables (printable ASCII)
ASC = ["ABCDEFGHIJKLM", "NOPQRSTUVWXYZ", "abcdefghijklm", "nopqrstuvwxyz", "0123456789 !#$%&()*+,-./:;=?@[]^_`{|}"]
for f in ["ar", "vd", "hv", "tn", "cn", "ss"]:
    for st in ["plain", "italic", "bold", "bolditalic"]:
        pre = {"plain": "", "italic": "skinparam componentFontStyle italic\n", "bold": "skinparam componentFontStyle bold\n",
               "bolditalic": "<style>\ncomponent {\n  FontStyle italic\n  FontWeight bold\n}\n</style>\n"}[st]
        body = "".join(f'component "{s}" as E{i}\n' for i, s in enumerate(ASC))
        add(f"c10a_ft_{f}_{st}", body, ["face-table", f, st], f, pre)
# true bold-italic through creole inside a bold label (FontConfiguration.add)
for f in ["ar", "vd", "tn"]:
    add(f"c10a_ft_{f}_creolebi", 'component "<i>Pylon wq vista Kiln</i>" as E1\n', ["face-table", f, "bold label + creole italic"], f,
        "skinparam componentFontStyle bold\n")
    add(f"c10a_ft_{f}_creolebi_ctrl", 'component "Pylon wq vista Kiln" as E1\n', ["face-table", f, "bold label", "control"], f,
        "skinparam componentFontStyle bold\n")

# sz: size plus italic
SZ = {"comp": ("component", 20, 'component "Kiln wq vista" as E1\n'), "part": ("participant", 18, 'participant "Kiln wq vista" as E1\nE1 -> E1 : ok\n'),
      "actv": ("activity", 17, 'start\n:Kiln wq vista;\n'), "stat": ("state", 19, 'state "Kiln wq vista" as E1\n'),
      "usec": ("usecase", 16, 'usecase "Kiln wq vista" as E1\n')}
for k, (kind, sz, body) in SZ.items():
    add(f"c10a_sz_{k}_p", body, ["size", k, str(sz), "plain", "control"], "tn", f"skinparam {kind}FontSize {sz}\n")
    add(f"c10a_sz_{k}_i", body, ["size", k, str(sz), "italic"], "tn", f"skinparam {kind}FontSize {sz}\nskinparam {kind}FontStyle italic\n")

# sx: state specifics; ax: activity specifics
SX = {
 "composite": 'state "Kiln wq vista" as E1 {\n  state Inner\n}\n',
 "descs": 'state E1\nE1 : first line\nE1 : a much longer second line\nE1 : third\n',
 "concurrent": 'state E1 {\n  state "Kiln wq" as A\n  --\n  state "Vista long" as B\n}\n',
 "longname": 'state "A considerably longer state name" as E1\nE1 --> E2\nstate "Second" as E2\n',
 "statename": 'state "Kiln wq vista" as E1\n',
}
for k, body in SX.items():
    add(f"c10a_sx_{k}_p", body, ["state", k, "plain", "control"], "tn")
    add(f"c10a_sx_{k}_i", body, ["state", k, "italic"], "tn", "skinparam stateFontStyle italic\n")
    add(f"c10a_sx_{k}_nm_i", body, ["state", k, "stateFontName + italic"], None, "skinparam stateFontName Times New Roman\nskinparam stateFontStyle italic\n")
    add(f"c10a_sx_{k}_ar_i", body, ["state", k, "Arial italic"], "ar", "skinparam stateFontStyle italic\n")
AX = {
 "table": 'start\n:|= Head |= Col |\n| kiln | wq vista long |;\n',
 "bullets": 'start\n:* first item\n* second longer item;\n',
 "numbered": 'start\n:# first\n# second longer item;\n',
 "repeat": 'start\nrepeat\n  :Kiln;\nbackward:Long wq vista backward;\nrepeat while (more?)\n',
 "deprecated": 'start\n#pink:Kiln wq vista;\n',
 "while": 'start\nwhile (more?)\n  :Kiln wq vista;\nendwhile\n',
}
for k, body in AX.items():
    add(f"c10a_ax_{k}_p", body, ["activity", k, "plain", "control"], "tn")
    add(f"c10a_ax_{k}_i", body, ["activity", k, "italic"], "tn", "skinparam activityFontStyle italic\n")
    add(f"c10a_ax_{k}_vd_i", body, ["activity", k, "Verdana italic"], "vd", "skinparam activityFontStyle italic\n")

json.dump({"batch": "C10A", "cases": cases}, open(D / "cases.json", "w"), indent=1, ensure_ascii=False)
print(len(cases), "cases")
