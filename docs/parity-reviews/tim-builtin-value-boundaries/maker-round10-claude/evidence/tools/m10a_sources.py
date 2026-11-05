"""Round-10 maker probes (M10A): <kind>FontStyle italic / bold / bold italic, measured-in-drawn-face.

Usage: python3 -I m10a_sources.py <outdir>
Kinds: component, node, rectangle, artifact, database alone (description diagram; database alone is a sequence
participant) and beside a component (nodc/rctc/artc/dbsc), actor beside a component (actd), a use case (actu) and in a
sequence (acts), usecase, state, participant, class, activity.
Axes per kind: Courier New with an unmapped mark (U+1D167) and its mark-free control; plain Latin in
the default family, Times New Roman (four distinct face tables) and Verdana; each with a no-FontStyle control.
Labels are renamed from BR ("Wharf q.r") to "Pylon v.k" so no case reuses a BR label.
"""
import json, pathlib, sys

OUT = pathlib.Path(sys.argv[1]); OUT.mkdir(parents=True, exist_ok=True)
MARK = "\U0001D167"
# kind -> (skinparam prefix, element line template, optional extra lines)
KINDS = {
    "comp": ("component", 'component "{L}" as E1', []),
    "node": ("node", 'node "{L}" as E1', []),
    "rect": ("rectangle", 'rectangle "{L}" as E1', []),
    "artf": ("artifact", 'artifact "{L}" as E1', []),
    "dbse": ("database", 'database "{L}" as E1', []),
    "actd": ("actor", 'actor "{L}" as E1', ['component "Ok" as E2', "E1 --> E2"]),
    "nodc": ("node", 'node "{L}" as E1', ['component "Ok" as E2', "E1 --> E2"]),
    "rctc": ("rectangle", 'rectangle "{L}" as E1', ['component "Ok" as E2', "E1 --> E2"]),
    "artc": ("artifact", 'artifact "{L}" as E1', ['component "Ok" as E2', "E1 --> E2"]),
    "dbsc": ("database", 'database "{L}" as E1', ['component "Ok" as E2', "E1 --> E2"]),
    "actu": ("actor", 'actor "{L}" as E1', ['usecase "Ok" as E2', "E1 --> E2"]),
    "usec": ("usecase", 'usecase "{L}" as E1', []),
    "stat": ("state", 'state "{L}" as E1', []),
    "part": ("participant", 'participant "{L}" as E1', ["E1 -> E1 : ok"]),
    "acts": ("actor", 'actor "{L}" as E1', ["E1 -> E1 : ok"]),
    "clss": ("class", 'class "{L}" as E1', []),
    "actv": ("activity", ":{L};", []),
}
STYLES = {"i": "italic", "b": "bold", "bi": "bold italic"}
FACES = {  # face tag -> (defaultFontName or None, label with/without mark)
    "cn": ("Courier New", "Pylon v" + MARK + "k", "Pylon vk"),
    "ss": (None, "Pylon vk", None),
    "tn": ("Times New Roman", "Pylon vk", None),
    "vd": ("Verdana", "Pylon vk", None),
}
cases = []
def emit(name, family, style_kw, prefix, elem, extra, label, axes):
    lines = ["@startuml"]
    if family:
        lines.append(f"skinparam defaultFontName {family}")
    if style_kw:
        lines.append(f"skinparam {prefix}FontStyle {style_kw}")
    if elem.startswith(":"):
        lines.append("start")
    lines.append(elem.format(L=label))
    lines += extra
    lines.append("@enduml")
    (OUT / f"{name}.puml").write_text("\n".join(lines) + "\n", encoding="utf-8")
    cases.append(dict(name=name, intended_valid=True, axes=axes))

for k, (prefix, elem, extra) in KINDS.items():
    for f, (family, label, ctrl_label) in FACES.items():
        emit(f"m10a_{k}_{f}_p", family, None, prefix, elem, extra, label, [k, f, "plain", "control"])
        if ctrl_label:
            emit(f"m10a_{k}_{f}_p_ctrl", family, None, prefix, elem, extra, ctrl_label, [k, f, "plain", "mark-free", "control"])
        for s, kw in STYLES.items():
            emit(f"m10a_{k}_{f}_{s}", family, kw, prefix, elem, extra, label, [k, f, kw])
            if ctrl_label:
                emit(f"m10a_{k}_{f}_{s}_ctrl", family, kw, prefix, elem, extra, ctrl_label, [k, f, kw, "mark-free", "control"])

# <style> FontStyle (Style.getUFont through the CSS path) for three kinds.
STYLE_SEL = {"comp": "component", "node": "node", "stat": "state", "usec": "usecase", "clss": "class", "part": "participant", "actv": "activity"}
for k, sel in STYLE_SEL.items():
    prefix, elem, extra = KINDS[k]
    for s, kw in (("i", "italic"), ("b", "bold")):
        for f in ("cn", "tn"):
            family, label, ctrl_label = FACES[f]
            name = f"m10a_css_{k}_{f}_{s}"
            lines = ["@startuml", f"skinparam defaultFontName {family}", "<style>", f"{sel} {{", f"  FontStyle {kw}", "}", "</style>"]
            if elem.startswith(":"):
                lines.append("start")
            lines += [elem.format(L=label)] + extra + ["@enduml"]
            (OUT / f"{name}.puml").write_text("\n".join(lines) + "\n", encoding="utf-8")
            cases.append(dict(name=name, intended_valid=True, axes=[k, f, kw, "style-block"]))

# Stereotype measured in the italic face it is drawn in (component kinds draw the stereotype italic).
for k in ("comp", "node", "dbse"):
    prefix, elem, extra = KINDS[k]
    for f in ("cn",):
        family, label, ctrl_label = FACES[f]
        for tag, st in (("mk", "Pier" + MARK), ("ctrl", "Pier")):
            name = f"m10a_stereo_{k}_{f}_{tag}"
            e = elem.format(L="Pylon vk") + f" <<{st}>>"
            lines = ["@startuml", f"skinparam defaultFontName {family}", e] + extra + ["@enduml"]
            (OUT / f"{name}.puml").write_text("\n".join(lines) + "\n", encoding="utf-8")
            cases.append(dict(name=name, intended_valid=True, axes=[k, f, "stereotype"] + (["control"] if tag == "ctrl" else [])))
(OUT.parent / "cases.json").write_text(json.dumps({"batch": "M10A", "cases": cases}, indent=1, ensure_ascii=False))
print(len(cases))
