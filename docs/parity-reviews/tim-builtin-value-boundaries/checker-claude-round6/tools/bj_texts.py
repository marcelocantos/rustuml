"""Per case: viewBox and <text> (textLength, content) for Java, base, merge, tip. Usage: python3 -I bj_texts.py <out dir> <batch json> [names...]"""
import json, pathlib, re, sys
O = pathlib.Path(sys.argv[1]); rows = json.load(open(sys.argv[2])); names = sys.argv[3:] or [n for n, r in rows.items() if r["tip_strict"] != "MATCH"]
def info(p):
    s = p.read_bytes().decode("utf-8", "replace") if p.exists() else ""
    vb = re.search(r'viewBox="([^"]*)"', s)
    t = [(a, b.replace("\n", "\\n").replace("\r", "\\r")) for a, b in re.findall(r'<text[^>]*?textLength="([^"]*)"[^>]*>(.*?)</text>', s, re.S)]
    return (vb.group(1) if vb else None), t
for n in names:
    print("==", n, "java_exit", rows[n]["java_exit"], rows[n]["java_stderr"][:150].replace("\n", " | "))
    jv, jt = info(O / f"{n}.java.svg")
    for tag in ["java", "base", "merge", "tip"]:
        v, t = info(O / f"{n}.{tag}.svg")
        print(f"  {tag:5s} vb={v} {t[:9]}")
