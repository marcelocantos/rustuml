"""Checker round-7 held-out batch C7B: JDK bidi components (TextLine.createComponentsOnRun -> GlyphLayout
script runs per component) and HarfBuzz >= 11.0.0's GDEF blocklist for macOS Courier New. Usage: c7b_sources.py <dir>"""
import json, pathlib, sys
B = pathlib.Path(sys.argv[1]); S = B / "sources"; S.mkdir(parents=True, exist_ok=True)
M = "\U0001D167"; cases = []
def add(name, body, axes, valid=True):
    (S / f"{name}.puml").write_bytes(f"@startuml\n{body}\n@enduml\n".encode()); cases.append({"name": name, "intended_valid": valid, "axes": axes})
def cls(label, font=None, bold=False):
    return (f"skinparam defaultFontName {font}\n" if font else "") + ("skinparam classFontStyle bold\n" if bold else "") + f'class "{label}" as K'
# bidi: neutral + mark between two RTL letters (one RTL component, Hebrew run) vs after the last RTL letter (LTR component)
add("c7b_verdana_rtl_space", cls(f"Wharf א {M}ב", "Verdana"), ["bidi", "rtl-component", "hebrew-run", "verdana"])
add("c7b_verdana_rtl_space_ctrl", cls("Wharf א ב", "Verdana"), ["control", "verdana"])
add("c7b_verdana_tail_space", cls(f"Wharf א {M}", "Verdana"), ["bidi", "ltr-component", "verdana"])
add("c7b_verdana_tail_space_ctrl", cls("Wharf א", "Verdana"), ["control", "verdana"])
add("c7b_verdanabold_rtl_space", cls(f"Wharf א {M}ב", "Verdana", True), ["bidi", "rtl-component", "hebrew-run", "verdana-bold"])
add("c7b_verdanabold_rtl_space_ctrl", cls("Wharf א ב", "Verdana", True), ["control", "verdana-bold"])
add("c7b_verdana_digit_rtl", cls(f"Wharf א7{M}ב", "Verdana"), ["bidi", "en-level-2", "verdana"])
add("c7b_verdana_digit_rtl_ctrl", cls("Wharf א7ב", "Verdana"), ["control", "verdana"])
add("c7b_verdana_arabic_rtl_space", cls(f"Wharf ب {M}ب", "Verdana"), ["bidi", "rtl-component", "arabic-run", "verdana", "gpos"])
add("c7b_verdana_arabic_rtl_space_ctrl", cls("Wharf ب ب", "Verdana"), ["control", "verdana"])
add("c7b_arial_rtl_space", cls(f"Wharf א {M}ב", "Arial"), ["bidi", "rtl-component", "hebrew-run", "arial"])
add("c7b_arial_rtl_space_ctrl", cls("Wharf א ב", "Arial"), ["control", "arial"])
add("c7b_helv_rtl_space", cls(f"Wharf א {M}ב", "Helvetica"), ["bidi", "rtl-component", "hebrew-run", "helv"])
add("c7b_helv_rtl_space_ctrl", cls("Wharf א ב", "Helvetica"), ["control", "helv"])
# whole-string script Thai/Devanagari (dumber: kept) vs component script Common (default: zeroed) on Lucida Grande
add("c7b_thai_rlm_digit", cls(f"Wharf ก‏7{M}"), ["bidi", "rlm", "thai-vs-common", "lucida"])
add("c7b_thai_rlm_digit_ctrl", cls("Wharf ก‏7"), ["control", "lucida"])
add("c7b_deva_rlm_digit", cls(f"Wharf क‏7{M}"), ["bidi", "rlm", "indic-vs-common", "lucida"])
add("c7b_deva_rlm_digit_ctrl", cls("Wharf क‏7"), ["control", "lucida"])
# Courier New: GDEF blocklisted (588, 5078, 14418) -> synthesized glyph classes -> unmapped Mn zeroed by GDEF
add("c7b_cnew_latin_mark", cls(f"Wharf x{M}", "Courier New"), ["gdef-blocklist", "default-run", "couriernew"])
add("c7b_cnew_latin_mark_ctrl", cls("Wharf x", "Courier New"), ["control", "couriernew"])
add("c7b_cnew_leading_mark", cls(f"{M}Wharf", "Courier New"), ["gdef-blocklist", "no-base", "couriernew"])
add("c7b_cnew_mc", cls("Wharf x\U00011F03", "Courier New"), ["gdef-blocklist", "mc", "couriernew"])
add("c7b_cnew_thai_run", cls(f"Wharf ก {M}", "Courier New"), ["gdef-blocklist", "thai-run", "couriernew"])
add("c7b_cnew_thai_run_ctrl", cls("Wharf ก", "Courier New"), ["control", "couriernew"])
add("c7b_cnewbold_latin_mark", cls(f"Wharf x{M}", "Courier New", True), ["gdef-blocklist", "couriernew-bold"])
add("c7b_cnewbold_latin_mark_ctrl", cls("Wharf x", "Courier New", True), ["control", "couriernew-bold"])
(B / "cases.json").write_text(json.dumps({"schema_version": 1, "batch": "C7B", "cases": cases}, indent=1, ensure_ascii=False))
print(len(cases), "cases")
