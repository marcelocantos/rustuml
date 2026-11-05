"""Measured-vs-drawn style scan (private probe build: family_text_width logs every call when RUSTUML_PROBE_WIDTHS is set).

Usage: python3 -I style_scan.py <probe-binary> <sources-dir> [<name-glob>]
For each label text holding 'Pylon' or 'Pier': the (bold, italic) styles it is drawn in (SVG <text> font-weight /
font-style) and the styles family_text_width measured it in. Reports texts measured in a style they are not drawn in.
"""
import html, json, pathlib, re, subprocess, sys
BIN, SRC = sys.argv[1], pathlib.Path(sys.argv[2]); GLOB = sys.argv[3] if len(sys.argv) > 3 else "*.puml"
T = re.compile(r"<text([^>]*)>(.*?)</text>", re.S)
out = {}
for p in sorted(SRC.glob(GLOB)):
    r = subprocess.run([BIN, "-pipe", "-tsvg"], input=p.read_bytes(), capture_output=True, timeout=60,
                       env={"PATH": "/usr/bin:/bin", "RUSTUML_PROBE_WIDTHS": "1"})
    svg = r.stdout.decode("utf-8", "replace")
    drawn = {}
    for attrs, body in T.findall(svg):
        t = html.unescape(body)
        if "Pylon" not in t and "Pier" not in t:
            continue
        st = ("bold" in attrs and 'font-weight="bold"' in attrs or 'font-weight="700"' in attrs,
              'font-style="italic"' in attrs)
        fam = re.search(r'font-family="([^"]*)"', attrs)
        drawn.setdefault(t.strip("«»"), set()).add((st, fam.group(1) if fam else None))
    measured = {}
    for line in r.stderr.decode("utf-8", "replace").splitlines():
        if not line.startswith("PROBE_W\t"):
            continue
        _, text, size, bold, italic, fam = line.split("\t")
        text = json.loads(text) if text.startswith('"') and "\\u{" not in text else text.strip('"')
        text = re.sub(r"\\u\{([0-9a-f]+)\}", lambda m: chr(int(m.group(1), 16)), text) if isinstance(text, str) else text
        if "Pylon" not in text and "Pier" not in text:
            continue
        measured.setdefault(text.strip("«»"), set()).add(((bold == "true", italic == "true"), fam))
    bad = {}
    for t, ms in measured.items():
        ds = {d[0] for d in drawn.get(t, set())}
        extra = sorted({m[0] for m in ms} - ds)
        if extra:
            bad[t] = dict(drawn=sorted(ds), measured=sorted({m[0] for m in ms}), families=sorted({m[1] for m in ms}))
    out[p.stem] = dict(exit=r.returncode, mismatches=bad)
    flag = "MISMATCH" if bad else "ok"
    print(f"{p.stem:28s} {flag} {json.dumps(bad, ensure_ascii=False) if bad else ''}")
json.dump(out, open(pathlib.Path.cwd() / "style-scan.json", "w"), indent=1, ensure_ascii=False)
