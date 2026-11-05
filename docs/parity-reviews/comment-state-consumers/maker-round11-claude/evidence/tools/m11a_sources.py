"""Round-11 maker held-out batch M11A: renamed Verdana italic / bold-italic probes (other labels, sizes, contexts),
each with its plain (or upright-bold) control, plus Arial italic regressions.
Usage: python3 -I m11a_sources.py <batch_dir>  -> <batch_dir>/sources/*.puml and cases.json

Groups: kd  element kind via skinparam <kind>FontStyle (italic, bold italic), label with the italic-only advances
            (A D V W g k m v y) and the bold-italic ones (o v);
        sz  defaultFontSize 11 / 17;
        cr  Creole <i> / <b><i> inside an upright Verdana label;
        cs  <style> FontStyle italic (+ FontWeight bold);
        ml  multi-line labels;
        ar  Arial italic regressions (Arial italics share the upright advances)."""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); (D / "sources").mkdir(parents=True, exist_ok=True)
cases = []
def add(name, body, axes, control=None):
    (D / "sources" / f"{name}.puml").write_text("@startuml\n" + body + "@enduml\n")
    cases.append({"name": name, "intended_valid": True, "axes": axes, "control": control})
L_I = "Dawn Vow gym kit"          # A D V W g k m v y-bearing (italic-only advances)
L_BI = "Ovo vow cove"             # o v (bold-italic-only advances)
KINDS = {
    "comp": ("component", 'component "{l}" as E1\n'),
    "node": ("node", 'node "{l}" as E1\n'),
    "artf": ("artifact", 'artifact "{l}" as E1\n'),
    "dbse": ("database", 'database "{l}" as E1\n'),
    "usec": ("usecase", 'usecase "{l}" as E1\n'),
    "actu": ("actor", 'actor "{l}" as E1\nusecase "x" as U1\nE1 --> U1\n'),
    "stat": ("state", 'state "{l}" as E1\n'),
    "part": ("participant", 'participant "{l}" as E1\nE1 -> E1 : ok\n'),
    "queu": ("participant", 'queue "{l}" as E1\nE1 -> E1 : ok\n'),
    "clss": ("class", 'class "{l}" as E1\n'),
    "actv": ("activity", 'start\n:{l};\n'),
}
for k, (sp, el) in KINDS.items():
    # 'bold italic' as a FontStyle value is plain in Java (ValueImpl.asFontFace), so the bold-italic face is
    # reached through the italic style and a Creole <b> label; its control is the same <b> label upright.
    for lab, tag, style in [(L_I, "i", "italic"), ("<b>" + L_BI + "</b>", "bi", "italic"), (L_I, "p", None), ("<b>" + L_BI + "</b>", "b", None)]:
        body = "skinparam defaultFontName Verdana\n" + (f"skinparam {sp}FontStyle {style}\n" if style else "") + el.format(l=lab)
        ctrl = None if tag in ("p", "b") else f"m11a_kd_{k}_vd_{'p' if tag == 'i' else 'b'}"
        add(f"m11a_kd_{k}_vd_{tag}", body, ["kind", k, "vd", tag] + (["control"] if ctrl is None else []), ctrl)
for size in (11, 17):
    for k in ("comp", "part", "actv", "stat"):
        sp, el = KINDS[k]
        for tag, style in (("i", "italic"), ("p", None)):
            body = (f"skinparam defaultFontName Verdana\nskinparam defaultFontSize {size}\n"
                    + (f"skinparam {sp}FontStyle {style}\n" if style else "") + el.format(l="Mew Ghyll AVW"))
            add(f"m11a_sz{size}_{k}_vd_{tag}", body, ["size", str(size), k, "vd", tag] + (["control"] if tag == "p" else []),
                None if tag == "p" else f"m11a_sz{size}_{k}_vd_p")
for k in ("comp", "part", "actv", "usec"):
    sp, el = KINDS[k]
    for tag, lab in (("i", "Tidy <i>Wavy kingdom</i> end"), ("bi", "<b>Rook <i>ovoid vow</i></b> end"), ("p", "Tidy Wavy kingdom end"), ("b", "<b>Rook ovoid vow</b> end")):
        body = "skinparam defaultFontName Verdana\n" + el.format(l=lab)
        ctrl = {"i": f"m11a_cr_{k}_vd_p", "bi": f"m11a_cr_{k}_vd_b"}.get(tag)
        add(f"m11a_cr_{k}_vd_{tag}", body, ["creole", k, "vd", tag] + (["control"] if ctrl is None else []), ctrl)
for k, sel in (("comp", "component"), ("usec", "usecase"), ("stat", "state")):
    _, el = KINDS[k]
    for tag, decl in (("i", "  FontStyle italic\n"), ("bi", "  FontStyle italic\n  FontWeight bold\n"), ("p", "  FontStyle normal\n")):
        body = f"skinparam defaultFontName Verdana\n<style>\n{sel} {{\n{decl}}}\n</style>\n" + el.format(l="Gawky Vandyke mop")
        add(f"m11a_cs_{k}_vd_{tag}", body, ["style", k, "vd", tag] + (["control"] if tag == "p" else []),
            None if tag == "p" else f"m11a_cs_{k}_vd_p")
for k in ("comp", "part", "stat"):
    sp, el = KINDS[k]
    for tag, style in (("i", "italic"), ("p", None)):
        lab = "Avowed yak\\nkindly Wigmore dew" if k != "stat" else "Avowed yak"
        extra = "E1 : kindly Wigmore dew\nE1 : vagary DVD\n" if k == "stat" else ""
        body = "skinparam defaultFontName Verdana\n" + (f"skinparam {sp}FontStyle {style}\n" if style else "") + el.format(l=lab) + extra
        add(f"m11a_ml_{k}_vd_{tag}", body, ["multiline", k, "vd", tag] + (["control"] if tag == "p" else []),
            None if tag == "p" else f"m11a_ml_{k}_vd_p")
for k in ("comp", "part", "actv"):
    sp, el = KINDS[k]
    for tag, style in (("i", "italic"), ("bi", "bold italic"), ("p", None)):
        body = "skinparam defaultFontName Arial\n" + (f"skinparam {sp}FontStyle {style}\n" if style else "") + el.format(l=L_I)
        add(f"m11a_ar_{k}_{tag}", body, ["arial", k, tag] + (["control"] if tag == "p" else []), None if tag == "p" else f"m11a_ar_{k}_p")
json.dump({"batch": "M11A", "cases": cases}, open(D / "cases.json", "w"), indent=1)
print(len(cases), "cases")
