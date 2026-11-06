"""Round-8 maker probes M8A (written before any Java result). Usage: python3 -I m8a_sources.py <batch_dir>
Each case has a control without the mark where the mark's advance is the measured quantity."""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); (D / "sources").mkdir(parents=True, exist_ok=True)
M, M2 = "\U0001D167", "\U0001D17B"
def cls(label, font=None, bold=False, extra=""):
    head = "@startuml\n" + (f"skinparam defaultFontName {font}\n" if font else "") + ("skinparam classFontStyle bold\n" if bold else "") + extra
    return head + f'class "{label}" as K\n@enduml\n'
cases = []
def add(name, text, axes, ctrl=None):
    cases.append((name, text, axes))
    if ctrl is not None:
        cases.append((name + "_ctrl", ctrl, axes + ["control"]))
# Obligation 1: bidi components. AL + EN (W2: EN -> AN).
add("m8b_lucida_arabic_digit", cls(f"Pier ب 7{M}"), ["bidi", "AL+EN"], cls("Pier ب 7"))
add("m8b_verdana_arabic_digit", cls(f"Pier ب7{M}", "Verdana"), ["bidi", "AL+EN"], cls("Pier ب7", "Verdana"))
add("m8b_verdana_heb_alm_digit", cls(f"Pier ב؜7{M}", "Verdana"), ["bidi", "AL+EN", "ALM"], cls("Pier ב؜7", "Verdana"))
add("m8b_lucida_thai_alm_digit", cls(f"Pier ข؜3{M}"), ["bidi", "AL+EN", "ALM"], cls("Pier ข؜3"))
add("m8b_lucida_thai_arabic_digit", cls(f"Pier ข ب4{M}"), ["bidi", "AL+EN"], cls("Pier ข ب4"))
add("m8b_verdana_heb_alm_es", cls(f"Pier ב؜7+8{M}", "Verdana"), ["bidi", "AL+EN", "W4"], cls("Pier ב؜7+8", "Verdana"))
add("m8b_helv_arabic_digit", cls(f"Pier س5{M}", "Helvetica"), ["bidi", "AL+EN"], cls("Pier س5", "Helvetica"))
# Explicit embeddings and overrides.
add("m8b_lucida_rle_digit", cls(f"Pier ข‫6{M}‬"), ["bidi", "RLE"], cls("Pier ข‫6‬"))
add("m8b_lucida_rlo_digit", cls(f"Pier ข‮6{M}‬"), ["bidi", "RLO"], cls("Pier ข‮6‬"))
add("m8b_verdana_lro_space", cls(f"Pier ב‭ {M}‬ג", "Verdana"), ["bidi", "LRO"], cls("Pier ב‭ ‬ג", "Verdana"))
add("m8b_verdana_lre_hebrew", cls(f"Pier ‪ב {M}‬ג", "Verdana"), ["bidi", "LRE"], cls("Pier ‪ב ‬ג", "Verdana"))
add("m8b_lucida_rli_with_hebrew", cls(f"Pier ב ข⁧6{M}⁩"), ["bidi", "RLI"], cls("Pier ב ข⁧6⁩"))
add("m8b_lucida_rli_alone", cls(f"Pier ข⁧6{M}⁩"), ["bidi", "RLI", "requiresBidi"], cls("Pier ข⁧6⁩"))
add("m8b_lucida_fsi_hebrew", cls(f"Pier ข⁨ב6{M}⁩"), ["bidi", "FSI"], cls("Pier ข⁨ב6⁩"))
add("m8b_verdana_rlm_deva", cls(f"Pier क‏6{M}", "Verdana"), ["bidi", "RLM"], cls("Pier क‏6", "Verdana"))
# Brackets (N0) and JDK's NSM after a closing bracket.
add("m8b_verdana_heb_bracket_nsm", cls(f"Pier ב(ג){M}", "Verdana"), ["bidi", "N0", "NSM-after-bracket"], cls("Pier ב(ג)", "Verdana"))
add("m8b_verdana_heb_bracket_latin", cls(f"Pier ב(x){M}", "Verdana"), ["bidi", "N0"], cls("Pier ב(x)", "Verdana"))
add("m8b_verdanabold_heb_tail_comma", cls(f"Pier ב,{M}", "Verdana", bold=True), ["bidi", "CS"], cls("Pier ב,", "Verdana", bold=True))
add("m8b_verdana_heb_digits_tail", cls(f"Pier ב 12{M}z", "Verdana"), ["bidi", "EN"], cls("Pier ב 12z", "Verdana"))
add("m8b_arial_heb_tail_space", cls(f"Pier ב {M}", "Arial"), ["bidi"], cls("Pier ב ", "Arial"))
# Obligation 2: Courier New GDEF blocklist and the GSUB chosen script.
add("m8g_cnew_deva", cls(f"Pier क{M}", "Courier New"), ["gdef-blocklist", "gsub-latn", "indic"], cls("Pier क", "Courier New"))
add("m8g_cnew_mymr", cls(f"Pier က{M}", "Courier New"), ["gdef-blocklist", "gsub-latn", "myanmar"], cls("Pier က", "Courier New"))
add("m8g_cnew_tibt", cls(f"Pier ཀ{M}", "Courier New"), ["gdef-blocklist", "gsub-latn", "use"], cls("Pier ཀ", "Courier New"))
add("m8g_cnew_khmer", cls(f"Pier ក{M}", "Courier New"), ["gdef-blocklist", "khmer"], cls("Pier ក", "Courier New"))
add("m8g_cnew_hangul", cls(f"Pier 가{M}", "Courier New"), ["gdef-blocklist", "hangul"], cls("Pier 가", "Courier New"))
add("m8g_cnew_arabic", cls(f"Pier ب{M}", "Courier New"), ["gdef-blocklist", "arabic", "bidi"], cls("Pier ب", "Courier New"))
add("m8g_cnew_syriac", cls(f"Pier ܐ{M}", "Courier New"), ["gdef-blocklist", "syriac", "bidi"], cls("Pier ܐ", "Courier New"))
add("m8g_cnew_mc_after_mark", cls(f"Pier q{M}\U00011F03", "Courier New"), ["gdef-blocklist", "Mc"], cls("Pier q\U00011F03", "Courier New"))
add("m8g_cnewbold_deva", cls(f"Pier क{M}", "Courier New", bold=True), ["gdef-blocklist", "bold"], cls("Pier क", "Courier New", bold=True))
add("m8g_cnew_size13", cls(f"Pier {M2}k\U0001E5EE", '"Courier New"', extra="skinparam defaultFontSize 13\n"), ["gdef-blocklist", "size"], cls("Pier k", '"Courier New"', extra="skinparam defaultFontSize 13\n"))
add("m8g_arial_deva", cls(f"Pier क{M}", "Arial"), ["gdef-classes", "control-face"], cls("Pier क", "Arial"))
add("m8g_cnew_sequence", f"@startuml\nskinparam defaultFontName Courier New\nparticipant \"Dock x{M}\" as A\nA -> A : ok\n@enduml\n", ["gdef-blocklist", "sequence"])
out = []
for name, text, axes in cases:
    (D / "sources" / f"{name}.puml").write_text(text, encoding="utf-8")
    out.append({"name": name, "intended_valid": True, "axes": axes})
(D / "cases.json").write_text(json.dumps({"schema_version": 1, "cases": out}, indent=1, ensure_ascii=False))
print(len(out), "cases")
