"""Checker round-9 (claude) held-out batch C9A: sources + cases.json. Usage: python3 -I c9a_sources.py <dir>
Groups:
 fs = family x style x context: an unmapped mark (U+1D167) in each modelled family (Arial, Helvetica, Verdana,
      Monospaced -> Menlo, Courier New, Lucida Grande) and each style (creole plain/<b>/<i>/<b><i>), two contexts per
      (family, style) rotated over class member, sequence message, activity action, state description, use-case name,
      component name and class note; every case has a mark-free control (plain Latin: must not change merge -> tip).
 ns = native style routes (abstract/interface names, {abstract} members, defaultFontStyle, classFontStyle) and the
      family name "Menlo" itself.
 af = class link center labels under arrow font name/size/style variants (skinparam ArrowFont*, classArrowFont*,
      <style>), multi-line labels, link stereotypes, ortho/polyline, self links, roles, label arrows, and link labels
      in object/use-case/state/deployment/component diagrams (component is known residue T14.39).
 sp = U+2029 / U+0085 inside multi-line note, legend and activity bodies (never on a command's first/last line), in
      pure-RTL and mixed text, with the mark in the second paragraph."""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); (D / "sources").mkdir(parents=True, exist_ok=True)
M = "\U0001D167"
HE = "אב"; HE2 = "גד"
cases = []
def add(name, body, axes, ctrl=True):
    cases.append((name, body, axes))
    if ctrl:
        assert M in body, name
        cases.append((name + "_ctrl", body.replace(M, ""), axes + ["control"]))
def font(f):
    return f"skinparam defaultFontName {f}\n" if f else ""
FAM = [("ar", "Arial"), ("hv", "Helvetica"), ("vd", "Verdana"), ("mo", "Monospaced"), ("cn", "Courier New"), ("lg", "Lucida Grande")]
STY = [("p", "{}"), ("b", "<b>{}</b>"), ("i", "<i>{}</i>"), ("bi", "<b><i>{}</i></b>")]
def ctx(kind, f, label):
    h = "@startuml\n" + font(f)
    return h + {
        "class": f"class Quay\nQuay : {label}\n",
        "seq": f"participant Ann\nparticipant Bob\nAnn -> Bob : {label}\n",
        "act": f"start\n:{label};\nstop\n",
        "state": f"state Quay\nQuay : {label}\n",
        "uc": f'usecase "{label}" as U1\n',
        "comp": f'component "{label}" as C1\n',
        "note": f"class Quay\nnote right of Quay : {label}\n",
    }[kind] + "@enduml\n"
CTX = ["class", "seq", "act", "state", "uc", "comp", "note"]
for fi, (fk, fname) in enumerate(FAM):
    for si, (sk, fmt) in enumerate(STY):
        for k in (fi + si, fi + si + 3):
            kind = CTX[k % len(CTX)]
            label = fmt.format(f"Wharf q{M}r")
            add(f"c9a_fs_{fk}_{sk}_{kind}", ctx(kind, fname, label), ["family-style", fname, sk, kind])
# ns: native style routes
add("c9a_ns_ar_abstract", f'@startuml\n{font("Arial")}abstract class "Wharf q{M}r" as K\n@enduml\n', ["native-italic", "Arial", "abstract"])
add("c9a_ns_hv_abstract", f'@startuml\n{font("Helvetica")}abstract class "Wharf q{M}r" as K\n@enduml\n', ["native-italic", "Helvetica", "abstract"])
add("c9a_ns_vd_interface", f'@startuml\n{font("Verdana")}interface "Wharf q{M}r" as K\n@enduml\n', ["native-italic", "Verdana", "interface"])
add("c9a_ns_mo_abstract", f'@startuml\n{font("Monospaced")}abstract class "Wharf q{M}r" as K\n@enduml\n', ["native-italic", "Monospaced", "abstract"])
add("c9a_ns_cn_interface_bold", f'@startuml\n{font("Courier New")}skinparam classFontStyle bold\ninterface "Wharf q{M}r" as K\n@enduml\n', ["native-bold-italic", "Courier New", "interface", "classFontStyle"])
add("c9a_ns_vd_classbold", f'@startuml\n{font("Verdana")}skinparam classFontStyle bold\nclass "Wharf q{M}r" as K\n@enduml\n', ["native-bold", "Verdana", "classFontStyle"])
add("c9a_ns_hv_absmember", f'@startuml\n{font("Helvetica")}class K\nK : {{abstract}} Wharf q{M}r()\n@enduml\n', ["native-italic", "Helvetica", "abstract-member"])
add("c9a_ns_lg_absmember", f'@startuml\n{font("Lucida Grande")}class K\nK : {{abstract}} Wharf q{M}r()\n@enduml\n', ["native-italic", "Lucida Grande", "abstract-member", "derived-italic-T14.41"])
add("c9a_ns_ar_seq_dfs_italic", f'@startuml\n{font("Arial")}skinparam defaultFontStyle italic\nparticipant Ann\nAnn -> Ann : Wharf q{M}r\n@enduml\n', ["native-italic", "Arial", "defaultFontStyle", "seq"])
add("c9a_ns_vd_uc_dfs_bold", f'@startuml\n{font("Verdana")}skinparam defaultFontStyle bold\nusecase "Wharf q{M}r" as U1\n@enduml\n', ["native-bold", "Verdana", "defaultFontStyle", "uc"])
add("c9a_ns_cn_comp_italic", f'@startuml\n{font("Courier New")}skinparam componentFontStyle italic\ncomponent "Wharf q{M}r" as C1\n@enduml\n', ["native-italic", "Courier New", "componentFontStyle"])
for sk, fmt in STY:
    add(f"c9a_ns_menlo_{sk}", ctx("class", "Menlo", fmt.format(f"Wharf q{M}r")), ["family-name-Menlo", sk])
# af: class link labels in the arrow-style font
def link(pre, rel="A --> B : Pier quay", decl="class A\nclass B\n"):
    return f"@startuml\n{pre}{decl}{rel}\n@enduml\n"
af = {
 "c9a_af_cn_name": link("skinparam ArrowFontName Courier New\n"),
 "c9a_af_vd_name_size17": link("skinparam ArrowFontName Verdana\nskinparam ArrowFontSize 17\n"),
 "c9a_af_size9": link("skinparam ArrowFontSize 9\n"),
 "c9a_af_class_prefix_name": link("skinparam classArrowFontName Arial\n"),
 "c9a_af_class_prefix_size": link("skinparam classArrowFontSize 20\n"),
 "c9a_af_style_class_arrow": link("<style>\nclassDiagram {\n  arrow {\n    FontName Helvetica\n    FontSize 15\n  }\n}\n</style>\n"),
 "c9a_af_style_arrow_size": link("<style>\narrow {\n  FontSize 18\n}\n</style>\n"),
 "c9a_af_multiline_vd": link("skinparam ArrowFontName Verdana\n", rel="A --> B : Pier\\nquay much longer"),
 "c9a_af_multiline3_size16": link("skinparam ArrowFontSize 16\n", rel="A --> B : one\\ntwo wider line\\nthree"),
 "c9a_af_stereo_skinparam": link("skinparam ArrowFontSize<<use>> 20\n", rel="A --> B <<use>> : Pier quay"),
 "c9a_af_stereo_global_vd": link("skinparam ArrowFontName Verdana\n", rel="A --> B <<use>> : Pier quay"),
 "c9a_af_style_bold": link("skinparam ArrowFontStyle bold\n"),
 "c9a_af_style_italic": link("skinparam ArrowFontStyle italic\n"),
 "c9a_af_ortho_vd": link("skinparam linetype ortho\nskinparam ArrowFontName Verdana\n"),
 "c9a_af_polyline_size16": link("skinparam linetype polyline\nskinparam ArrowFontSize 16\n"),
 "c9a_af_lr_cn": link("left to right direction\nskinparam ArrowFontName Courier New\n"),
 "c9a_af_self_size17": link("skinparam ArrowFontSize 17\n", rel="A --> A : self quay", decl="class A\n"),
 "c9a_af_roles_size16": link("skinparam ArrowFontSize 16\n", rel='A "1" --> "many" B : Pier quay'),
 "c9a_af_labelarrow_vd": link("skinparam ArrowFontName Verdana\n", rel="A --> B : Pier quay >"),
 "c9a_af_creole_ar": link("skinparam ArrowFontName Arial\n", rel="A --> B : <b>Pier</b> quay"),
 "c9a_af_default_vd": link("skinparam defaultFontName Verdana\n"),
 "c9a_af_default_size16": link("skinparam defaultFontSize 16\n"),
 "c9a_af_plain": link(""),
 "c9a_af_package_size15": link("skinparam ArrowFontSize 15\n", decl="package P1 {\n  class A\n}\npackage P2 {\n  class B\n}\n"),
 "c9a_af_two_links": link("skinparam ArrowFontName Verdana\nskinparam ArrowFontSize 12\n", rel="A --> B : Pier quay\nB ..> C : wharf longer label", decl="class A\nclass B\nclass C\n"),
 "c9a_af_object_vd": link("skinparam ArrowFontName Verdana\n", decl="object A\nobject B\n"),
 "c9a_af_usecase_vd": link("skinparam ArrowFontName Verdana\n", decl="actor A\nusecase B\n"),
 "c9a_af_state_vd": link("skinparam ArrowFontName Verdana\n", rel="[*] --> B : Pier quay", decl="state B\n"),
 "c9a_af_deploy_size16": link("skinparam ArrowFontSize 16\n", decl="node A\nnode B\n"),
 "c9a_af_component_vd": link("skinparam ArrowFontName Verdana\n", rel="[A] --> [B] : Pier quay", decl=""),
}
for n, b in af.items():
    cases.append((n, b, ["arrow-font"]))
add("c9a_af_vd_mark", link("skinparam ArrowFontName Verdana\n", rel=f"A --> B : Wharf q{M}r"), ["arrow-font", "mark"])
add("c9a_af_cn_italic_mark", link("skinparam ArrowFontName Courier New\nskinparam ArrowFontStyle italic\n", rel=f"A --> B : Wharf q{M}r"), ["arrow-font", "mark", "arrow-font-style"])
add("c9a_af_ar_multiline_mark", link("skinparam ArrowFontName Arial\nskinparam ArrowFontSize 11\n", rel=f"A --> B : Pier\\nWharf q{M}r"), ["arrow-font", "mark", "multi-line"])
add("c9a_af_hv_size15_mark", link("skinparam classArrowFontName Helvetica\nskinparam classArrowFontSize 15\n", rel=f"A --> B : Wharf q{M}r"), ["arrow-font", "mark", "class-prefix"])
# sp: separators in multi-line bodies (never a command's first or last line)
SEP = {"P": " ", "N": "\u0085"}
FORMS = {
 "pdf": lambda s: f"{HE}{s}‬ {M}",
 "sp": lambda s: f"{HE}{s} {M}",
 "mk": lambda s: f"{HE}{s}{M}",
 "rr": lambda s: f"{HE}{s}{HE2}{M}",
 "mx": lambda s: f"Pier {HE}{s}{M} x",
 "dg": lambda s: f"{HE}{s}12{M}",
}
def body(kind, f, line):
    h = "@startuml\n" + font(f)
    return h + {
        "seqnote": f"participant Ann\nnote over Ann\nquay\n{line}\nend note\n",
        "seqnote_left": f"participant Ann\nnote left of Ann\nquay\n{line}\nend note\n",
        "classnote": f"class Quay\nnote right of Quay\nquay\n{line}\nend note\n",
        "floatnote": f"note as N1\nquay\n{line}\nend note\n",
        "actnote": f"start\n:quay;\nnote right\nquay\n{line}\nend note\nstop\n",
        "actmid": f"start\n:quay\n{line}\nwharf;\nstop\n",
        "statenote": f"state Quay\nnote right of Quay\nquay\n{line}\nend note\n",
        "legend": f"class Quay\nlegend\nquay\n{line}\nendlegend\n",
        "ucnote": f"usecase U1\nnote right of U1\nquay\n{line}\nend note\n",
    }[kind] + "@enduml\n"
SFONTS = ["Verdana", "Arial", None, "Courier New"]
i = 0
for fk in FORMS:
    for sk in SEP:
        f = SFONTS[i % len(SFONTS)]; i += 1
        add(f"c9a_sp_seqnote_{fk}_{sk}", body("seqnote", f, FORMS[fk](SEP[sk])), ["separator", "seqnote", fk, sk, f or "default"])
OTHER = ["seqnote_left", "classnote", "floatnote", "actnote", "actmid", "statenote", "legend", "ucnote"]
FK = list(FORMS)
for ci, kind in enumerate(OTHER):
    for j in range(2):
        fk = FK[(ci * 2 + j) % len(FK)]; sk = "PN"[(ci + j) % 2]; f = SFONTS[(ci + j) % len(SFONTS)]
        add(f"c9a_sp_{kind}_{fk}_{sk}", body(kind, f, FORMS[fk](SEP[sk])), ["separator", kind, fk, sk, f or "default"])
# sv: the discriminating form (second paragraph without a strong character, PDF before the mark) in Verdana, whose
# plan zeroes the mark only in the right-to-left component, in every body context and both separators
for kind in ["seqnote"] + OTHER:
    for sk in SEP:
        if kind == "seqnote" and sk == "P":
            continue  # c9a_sp_seqnote_pdf_P above
        add(f"c9a_sv_{kind}_{sk}", body(kind, "Verdana", FORMS["pdf"](SEP[sk])), ["separator", "discriminating", kind, sk, "Verdana"])
        add(f"c9a_sv_{kind}_{sk}_sp", body(kind, "Verdana", f"{HE}{SEP[sk]}\u202c\u202c {M}"), ["separator", "discriminating", "double-pdf", kind, sk, "Verdana"])
names = [c[0] for c in cases]; assert len(names) == len(set(names))
out = []
for n, b, axes in cases:
    (D / "sources" / f"{n}.puml").write_text(b, encoding="utf-8")
    out.append({"name": n, "intended_valid": True, "axes": axes})
(D / "cases.json").write_text(json.dumps({"batch": "C9A", "cases": out}, indent=1, ensure_ascii=False) + "\n")
print(len(out), "cases")
