"""Checker round-8 held-out batch C8A: sources + cases.json. Usage: python3 -I c8a_sources.py <dir>
Groups: ci = Courier New italic/bold-italic faces (not in the HarfBuzz 15.0.0 GDEF blocklist; RustUML measures every
Courier New style with the blocklisted plain face); cr = Courier New regular positive controls in the same routes;
hm = Helvetica U+2E04/U+2E05 bidi mirroring (409 vs 408 units); bl = bidi level -> TextLine component -> script
run -> zeroing (no format controls unless named); tn = Times New Roman Italic (blocklisted on the host);
lt = pure-LTR complex text across diagram families (no RTL: the bidi path must be a no-op)."""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); (D / "sources").mkdir(parents=True, exist_ok=True)
M = "\U0001D167"   # MUSICAL SYMBOL COMBINING TREMOLO-1, Mn, unmapped in every modelled face
HE, HE2, HE3 = "א", "ב", "ג"
AR = "ا"; TH = "ก"; DV = "क"; KH = "ក"; HG = "가"
cases = []
def add(name, body, axes, ctrl=True, valid=True):
    cases.append((name, body, axes, valid))
    if ctrl:
        cases.append((name + "_ctrl", body.replace(M, ""), axes + ["control"], valid))
def cls(label, font=None, extra="", kw="class", size=None):
    s = "@startuml\n"
    if font: s += f"skinparam defaultFontName {font}\n"
    if size: s += f"skinparam defaultFontSize {size}\n"
    s += extra
    return s + f'{kw} "{label}" as K\n@enduml\n'
CN = "Courier New"
# ci: Courier New italic routes (abstract class name, stereotype, defaultFontStyle, creole, bold italic)
add("c8a_ci_abstract_latin", cls(f"Pier x{M}", CN, kw="abstract class"), ["courier-italic", "abstract"])
add("c8a_ci_abstract_hebrew", cls(f"Pier {HE}{M}", CN, kw="abstract class"), ["courier-italic", "abstract", "hebrew"])
add("c8a_ci_abstract_leading", cls(f"{M}Pier", CN, kw="abstract class"), ["courier-italic", "abstract", "leading-mark"])
add("c8a_ci_stereo_latin", f"@startuml\nskinparam defaultFontName {CN}\nclass K <<Pier x{M}>>\n@enduml\n", ["courier-italic", "stereotype"])
add("c8a_ci_fontstyle_italic", cls(f"Pier x{M}", CN, extra="skinparam defaultFontStyle italic\n"), ["courier-italic", "defaultFontStyle"])
add("c8a_ci_creole_italic", f"@startuml\nskinparam defaultFontName {CN}\nclass K\nK : <i>Pier x{M}</i>\n@enduml\n", ["courier-italic", "creole-member"])
add("c8a_ci_creole_bolditalic", f"@startuml\nskinparam defaultFontName {CN}\nclass K\nK : <b><i>Pier x{M}</i></b>\n@enduml\n", ["courier-bold-italic", "creole-member"])
add("c8a_ci_usecase_italic", f"@startuml\nskinparam defaultFontName {CN}\nusecase \"<i>Pier x{M}</i>\" as U\n@enduml\n", ["courier-italic", "usecase"])
add("c8a_ci_abstract_size11", cls(f"Wharf q{M}r", CN, kw="abstract class", size=11), ["courier-italic", "abstract", "size"])
# cr: Courier New regular in the same routes (blocklisted plain face; Java zeroes)
add("c8a_cr_member_plain", f"@startuml\nskinparam defaultFontName {CN}\nclass K\nK : Pier x{M}\n@enduml\n", ["courier-regular", "member"])
add("c8a_cr_member_bold", f"@startuml\nskinparam defaultFontName {CN}\nclass K\nK : <b>Pier x{M}</b>\n@enduml\n", ["courier-bold", "creole-member"])
add("c8a_cr_usecase_plain", f"@startuml\nskinparam defaultFontName {CN}\nusecase \"Pier x{M}\" as U\n@enduml\n", ["courier-regular", "usecase"])
# hm: Helvetica mirroring of U+2E04/U+2E05 inside an RTL run (advances 409/408 of 2048)
L4, R5 = "⸄" * 6, "⸅" * 6
add("c8a_hm_rtl_left", cls(f"{HE}{L4}{HE2}", "Helvetica", size=60), ["mirroring", "helvetica"], ctrl=False)
add("c8a_hm_rtl_right", cls(f"{HE}{R5}{HE2}", "Helvetica", size=60), ["mirroring", "helvetica"], ctrl=False)
add("c8a_hm_ltr_left", cls(f"a{L4}b", "Helvetica", size=60), ["mirroring", "helvetica", "control"], ctrl=False)
add("c8a_hm_ltr_right", cls(f"a{R5}b", "Helvetica", size=60), ["mirroring", "helvetica", "control"], ctrl=False)
add("c8a_hm_lucida_parens", cls(f"{HE}(((({HE2}))))", None, size=60), ["mirroring", "lucida", "equal-advance"], ctrl=False)
# bl: levels -> components -> zeroing
add("c8a_bl_verdana_bracket_nsm_latin", cls(f"Pier {HE}({HE2}){M} x", "Verdana"), ["bidi", "N0", "NSM-after-bracket", "jdk-deviation"])
add("c8a_bl_verdana_square_nsm_end", cls(f"Pier {HE} [{HE2}]{M}", "Verdana"), ["bidi", "N0", "NSM-after-bracket"])
add("c8a_bl_verdana_angle_nsm_end", cls(f"Pier {HE}〈{HE2}〉{M}", "Verdana"), ["bidi", "N0", "canonical-bracket"])
add("c8a_bl_verdana_bracket_nsm_r", cls(f"Pier {HE}({HE2}){M}{HE3}", "Verdana"), ["bidi", "N0", "N1"])
add("c8a_bl_verdana_mark_then_digit", cls(f"Pier {HE}{M}1", "Verdana"), ["bidi", "R-run"])
add("c8a_bl_verdana_open_l_context", cls(f"Pier ({HE}){M}", "Verdana"), ["bidi", "N0", "embedding-direction"])
add("c8a_bl_lucida_an_digit_thai", cls(f"Pier {TH} ١{M}", None), ["bidi", "AN", "script-of-component"])
add("c8a_bl_lucida_al_en_thai", cls(f"Pier {TH}{AR}7{M}", None), ["bidi", "W2"])
add("c8a_bl_lucida_al_then_thai_en", cls(f"Pier {AR} {TH}7{M}", None), ["bidi", "W2", "W7"])
add("c8a_bl_lucida_thai_digit_after_r", cls(f"Pier {TH} {HE}๗{M}", None), ["bidi", "L-digit"])
add("c8a_bl_lucida_ext_arabic_digit", cls(f"Pier {TH} {HE}۷{M}", None), ["bidi", "EN", "arabic-script-digit"])
add("c8a_bl_lucida_en_cs_en", cls(f"Pier {TH} {HE}1,2{M}", None), ["bidi", "W4"])
add("c8a_bl_lucida_an_cs_an", cls(f"Pier {TH} {AR}1,2{M}", None), ["bidi", "W4", "AN"])
add("c8a_bl_lucida_et_en", cls(f"Pier {TH} {HE}$1{M}", None), ["bidi", "W5"])
add("c8a_bl_lucida_rli_digit", cls(f"Pier {TH}⁧7{M}⁩ {HE}", None), ["bidi", "RLI", "isolate"])
add("c8a_bl_lucida_deva_digit_r", cls(f"Pier {DV} {HE}७{M}", None), ["bidi", "L-digit", "deva"])
add("c8a_bl_cnew_khmer_split", cls(f"Pier {KH} {HE}7{M}", CN), ["bidi", "courier-regular", "khmer"])
add("c8a_bl_cnew_hangul_split", cls(f"Pier {HG} {HE}7{M}", CN), ["bidi", "courier-regular", "hangul"])
add("c8a_bl_verdana_sequence", f"@startuml\nskinparam defaultFontName Verdana\nparticipant \"Pier {HE}({HE2}){M} x\" as P\nP -> P : Quay {HE}1{M}\n@enduml\n", ["bidi", "sequence", "N0"])
add("c8a_bl_verdana_activity", f"@startuml\nskinparam defaultFontName Verdana\nstart\n:Pier {HE} {M}x;\n:Quay {HE}{M} x;\nstop\n@enduml\n", ["bidi", "activity"])
# tn: Times New Roman (Italic is blocklisted (490, 3046, 41638); regular is not)
add("c8a_tn_abstract_italic", cls(f"Pier x{M}", "Times New Roman", kw="abstract class"), ["times-italic", "blocklisted"])
add("c8a_tn_regular", cls(f"Pier x{M}", "Times New Roman"), ["times-regular"])
# lt: pure LTR complex text (no RTL) across families; merge and tip must be byte-identical
lt = {
 "c8a_lt_deva_participant": f"@startuml\nparticipant \"क्षत्रिय {M}\" as P\nP -> P : नमस्ते\n@enduml\n",
 "c8a_lt_thai_note": f"@startuml\nclass K\nnote right of K : สวัสดี {M}x\n@enduml\n",
 "c8a_lt_cjk_usecase": "@startuml\nusecase \"漢字かな €́\" as U\nactor \"한국어\" as A\nA --> U : テスト́\n@enduml\n",
 "c8a_lt_emoji_title": "@startuml\ntitle Pier \U0001F600\U0001F44D\U0001F3FD é\nclass K\n@enduml\n",
 "c8a_lt_latin_marks_state": f"@startuml\nstate \"Pier á̂ x{M}\" as S\n[*] --> S : q̈{M}\n@enduml\n",
 "c8a_lt_verdana_deva_link": f"@startuml\nskinparam defaultFontName Verdana\nclass A\nclass B\nA --> B : क{M} x\n@enduml\n",
 "c8a_lt_arial_tibetan": f"@startuml\nskinparam defaultFontName Arial\nclass \"Pier ཀ{M}\" as K\n@enduml\n",
 "c8a_lt_cnew_thai_stereo": f"@startuml\nskinparam defaultFontName Courier New\nclass K <<{TH}{M}>>\n@enduml\n",
}
for n, b in lt.items(): cases.append((n, b, ["ltr-invariance"], True))
out = []
for name, body, axes, valid in cases:
    (D / "sources" / f"{name}.puml").write_bytes(body.encode("utf-8")); out.append({"name": name, "intended_valid": valid, "axes": axes})
(D / "cases.json").write_text(json.dumps({"batch": "C8A", "cases": out}, indent=1, ensure_ascii=False))
print(len(out), "cases")
