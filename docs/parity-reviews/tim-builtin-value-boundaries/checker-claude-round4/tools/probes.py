"""Rust-only probe table (no Java run): exit, stderr, escape/carrier visibility, widths, border invariant.
Usage: python3 -I probes.py <probes_dir> <out.json>"""
import hashlib, json, pathlib, re, subprocess, sys
D = pathlib.Path(sys.argv[1]); OUT = sys.argv[2]
W = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round4-claude-checker"
TREES = {"base_3992971a7": "3992971a7", "round3_71d16da4a": "71d16da4a", "tip_7ec2e5af8": "7ec2e5af8"}
ENV = {"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": "date=1774210426000,tz=AEDT+1100", "TZ": "GMT+11:00"}
ESC = "\U0010F7FF"
out = {}
for group in ("carrier", "width", "state"):
    rows = {}
    for src in sorted((D / group).glob("*.puml")):
        s = src.read_text(encoding="utf-8"); row = {"source_sha256": hashlib.sha256(src.read_bytes()).hexdigest(), "source_text": s}
        for tag, rev in TREES.items():
            fmts = ("-tsvg", "--ast", "--yaml", "-ttxt") if group == "carrier" else ("-tsvg",)
            for fmt in fmts:
                p = subprocess.run([str(W / f"bin/rustuml-{rev}"), "-pipe", fmt], input=src.read_bytes(), capture_output=True, cwd=W / "work/cwd-c4a", env=ENV, timeout=60)
                t = p.stdout.decode("utf-8", "replace"); k = f"{tag}{fmt}"
                r = {"exit": p.returncode, "stdout_sha256": hashlib.sha256(p.stdout).hexdigest(), "stderr_head": p.stderr.decode("utf-8", "replace")[:160]}
                if fmt == "-tsvg":
                    r["textLength"] = re.findall(r'textLength="([^"]*)"', t)[:3]
                    r["raw_escape_visible"] = ESC in t and ESC not in s and "1112063" not in s
                    r["escape_ref_visible"] = ("&#1112063;" in t or "&#x10f7ff;" in t.lower()) and ESC not in s and "1112063" not in s
                else:
                    r["u_fffd_present"] = "�" in t
                    r["escape_visible"] = (ESC in t or "\\u{10f7ff}" in t) and ESC not in s and "1112063" not in s
                row[k] = r
        rows[src.stem] = row
    out[group] = rows
json.dump(out, open(OUT, "w"), indent=1, ensure_ascii=False)
for g, rows in out.items():
    print("==", g, len(rows))
