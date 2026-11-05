"""Pre-register the checker's C10A expectations (tip e24240b35) before any Java run. Usage: python3 -I c10a_expect.py <batch_dir>
Outcomes: match / fail / match-iff-control (named controls) / record (no claim)."""
import json, pathlib, sys
D = pathlib.Path(sys.argv[1]); cases = json.load(open(D / "cases.json"))["cases"]; names = {c["name"] for c in cases}
out = []
def E(n, outcome, why, controls=None): out.append({"name": n, "expect": outcome, "controls": controls or [], "hypothesis": why})
for c in cases:
    n, a = c["name"], c["axes"]; g = n.split("_")[1]
    if "control" in a: E(n, "control", "no FontStyle"); continue
    if g == "st":
        k, f = a[1], a[2]
        if k == "clss": E(n, "record", "class stereotype: outside the packet")
        elif f == "vd": E(n, "fail", "H1: stereotype drawn Verdana Italic, measured with the Verdana Regular ASCII table (family_table_text_width has no italic table)")
        elif f in ("ar", "cn", "ss"): E(n, "record", "H2/H3: Arial Italic, Courier New Italic and derived Lucida italic share their upright ASCII advances (unverified for Arial)")
        else: E(n, "match", "H4: Times stereotype now measured in Times Italic as drawn (EntityImageDescription fcStereo)")
    elif g == "cl": E(n, "record", "H7: cloud outline from the measured block (USymbolCloud.asSmall); no claim before Java")
    elif g == "ml":
        k, f, st = a[1], a[2], a[3]; ctl = f"c10a_ml_{k}_{f}_p"
        E(n, "fail" if (f == "vd" and st == "italic") else "match-iff-control", "H1 Verdana italic table gap" if (f == "vd" and st == "italic") else "multi-line label measured per line in the drawn face", [ctl])
    elif g == "cr": E(n, "match-iff-control", "Creole runs add to the block FontConfiguration (FontConfiguration.add)", [n[:-2] + "_p"])
    elif g == "pr": E(n, "record", "precedence/selector forms: no claim before Java", [f"c10a_pr_{a[1]}_ctrl"])
    elif g == "ns":
        k, f = a[1], a[2]; ctl = f"c10a_ns_{k}_tn_p"
        E(n, "match-iff-control", "nested element measured in its drawn face", [ctl])
    elif g == "fv": E(n, "match-iff-control", "ValueImpl.asFontFace value forms (Rust font_face) ", [f"c10a_fv_{a[1]}_ctrl"])
    elif g == "bi":
        k = a[1]; ctl = f"c10a_bi_{k}_p"
        E(n, "fail", "H5: 'bold italic' consumer outside the packet still reads contains('bold'/'italic') or ignores FontStyle; Java asFontFace gives plain", [ctl]) if k in ("classname", "classattr", "objname", "seqarrow", "comparrow", "title", "legend", "pkg", "seqbox", "swim", "compnote", "seqnote", "statattr") else E(n, "match-iff-control", "diamond: activityDiamondFontStyle now read by font_face", [ctl])
    elif g == "ft":
        f, st = a[1], a[2]
        if f == "vd" and st in ("italic", "bolditalic", "bold label + creole italic"): E(n, "fail", "H1: Verdana Italic/BoldItalic ASCII advances absent; Regular/Bold tables used")
        elif st == "bolditalic": E(n, "fail", "FontWeight not modelled (T14.46): Rust draws italic only")
        elif st in ("italic", "bold label + creole italic"): E(n, "match-iff-control", "italic face advances", [f"c10a_ft_{f}_plain" if st == "italic" else f"c10a_ft_{f}_creolebi_ctrl"])
        else: E(n, "record", "face table baseline")
    elif g == "sz": E(n, "match-iff-control", "size and italic in one FontConfiguration", [f"c10a_sz_{a[1]}_p"])
    elif g == "sx":
        k = a[1]
        if "Arial" in a[2]: E(n, "fail", "H8: state.rs arial_text_width measures Arial in SansSerif and drops italic (T14.47 path)")
        else: E(n, "match-iff-control", "state title/fields measured in the drawn face", [f"c10a_sx_{k}_p"])
    elif g == "ax":
        k = a[1]
        if "Verdana" in a[2]: E(n, "fail", "H1 Verdana italic table gap")
        else: E(n, "match-iff-control", "action text measured in the drawn face", [f"c10a_ax_{k}_p"])
    else: E(n, "record", "")
for e in out:
    for c in e["controls"]: assert c in names, (e["name"], c)
json.dump({"schema_version": 1, "registered_before_java": True, "tip": "e24240b3542aaabdaf6286b87c1cb23217898ddc", "cases": out}, open(D / "checker-expectations.json", "w"), indent=1)
import collections; print(collections.Counter(e["expect"] for e in out))
