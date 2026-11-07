"""Stage maker batch mk4b (round 4: removeInnerComment in the theme seed source, UmlSource) as an
immutable Java reference request.

Usage: python3 -I stage_mk4b.py BATCH_DIR
Same format as stage_mk4.py. No Java is run here. Validity is decided later from raw Java SVG + stderr.
"""
import base64, gzip, hashlib, json, sys
from pathlib import Path

BATCH = Path(sys.argv[1])
BATCH.mkdir(exist_ok=False)
INPUTS = BATCH / "inputs"
INPUTS.mkdir()
TH = "!theme cerulean\n"
cases = []


def add(name, body, valid, axes):
    cases.append((name, f"@startuml\n{body}@enduml\n", valid, axes))


add("mk4b_theme_trailing_comment_seq", TH + "Alder -> Birch : hi /' c '/\n", True,
    ["!theme seed source", "trailing /' '/", "sequence"])
add("mk4b_theme_trailing_comment_usecase", TH + ":Clerk: --> (Pay) /' c '/\n", True,
    ["!theme seed source", "trailing /' '/", "usecase"])
add("mk4b_theme_leading_comment_component", TH + "/' c '/ [Engine]\n[Engine] --> [Pump]\n", True,
    ["!theme seed source", "leading /' '/ (PLAIN, tail kept)", "component"])
add("mk4b_theme_triple_word_state", TH + "state \"Ready/'''x'''/Set\" as RS\n[*] --> RS\n", True,
    ["!theme seed source", "mid-line /'''word'''/", "state"])
add("mk4b_theme_comment_before_theme_class", "class Pine /' c '/\n" + TH, True,
    ["!theme after an inner-comment line", "trailing /' '/", "class"])
add("mk4b_theme_pixel_sprite_end_comment_object", TH + "sprite $dot [2x2/16] {\nF0\n0F\n} /' e '/\nobject Crate\n", True,
    ["!theme seed source", "pixel END trailing /' '/", "object"])
add("mk4b_theme_whole_line_comment_class", TH + "' note\nclass Pine\n", True,
    ["!theme seed source", "whole-line ' comment (residue: raw text kept in Rust seed source)", "class"])
add("mk4b_theme_block_comment_class", TH + "/' a\nb '/\nclass Pine\n", True,
    ["!theme seed source", "block comment (residue: raw text kept in Rust seed source)", "class"])

manifest_cases, files = [], []
for name, src, valid, axes in cases:
    assert all(c == "\n" or ord(c) >= 0x20 for c in src), name
    assert not any(0xE000 <= ord(c) <= 0xE0FF for c in src), name
    data = src.encode()
    p = INPUTS / f"{name}.puml"
    p.write_bytes(data)
    h = hashlib.sha256(data).hexdigest()
    manifest_cases.append({"name": name, "source": str(p), "source_sha256": h, "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
                           "invocation": ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone",
                                          "GMT+11:00", "--", "-pipe", "-tsvg"],
                           "env": {}, "intended_valid": valid, "axes": axes})
    files.append({"path": f"{name}.puml", "sha256": h, "base64": base64.b64encode(data).decode()})
assert len({c["name"] for c in manifest_cases}) == len(manifest_cases)
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
            "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": manifest_cases}
mbytes = json.dumps(manifest, indent=1, ensure_ascii=False).encode()
(BATCH / "reference-manifest.json").write_bytes(mbytes)
bundle = {"manifest": manifest, "manifest_sha256": hashlib.sha256(mbytes).hexdigest(), "files": files, "resources": []}
with gzip.GzipFile(BATCH / "reference-input-bundle.json.gz", "wb", mtime=0) as fh:
    fh.write(json.dumps(bundle, ensure_ascii=False).encode())
for f in ("reference-manifest.json", "reference-input-bundle.json.gz"):
    print(hashlib.sha256((BATCH / f).read_bytes()).hexdigest(), f)
print(len(manifest_cases), sum(c["intended_valid"] for c in manifest_cases), "intended valid")
