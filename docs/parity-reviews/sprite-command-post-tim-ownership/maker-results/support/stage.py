"""Stage an immutable Java reference request for the Claude checker probes."""
import base64, gzip, hashlib, json, pathlib, sys

ROOT = pathlib.Path(sys.argv[1])
BATCH = ROOT / "batch1"
INPUTS = BATCH / "inputs"
INPUTS.mkdir(parents=True, exist_ok=False)

SEQ = {"legacy": "", "teoz": "!pragma teoz true\n"}
cases = []

def add(name, body, valid, axes, mechanism):
    cases.append((name, "@startuml\n" + body + "@enduml\n", valid, axes, mechanism))

SB = "sequence-sprite-block-consumption"
for mode, pre in SEQ.items():
    add(f"cl_seq_note_upper_{mode}", pre + "participant Cedar\nparticipant Spruce\nnote over Cedar\nSprite sheets are loaded\nend note\nCedar -> Spruce: go\n", True, ["multiline note text", "Sprite capitalised", mode], SB)
    add(f"cl_seq_note_caps_{mode}", pre + "participant Maple\nparticipant Birch\nnote over Birch\nSPRITE RENDERER QUEUE DEPTH\nend note\nBirch -> Maple: ack\n", True, ["multiline note text", "SPRITE uppercase", mode], SB)
    add(f"cl_seq_note_lower_{mode}", pre + "participant Cedar\nparticipant Spruce\nnote over Cedar\nsprite sheets are loaded\nend note\nCedar -> Spruce: go\n", True, ["multiline note text", "sprite lowercase control", mode], SB)
    add(f"cl_seq_note_brace_{mode}", pre + "participant Cedar\nparticipant Spruce\nnote over Cedar\nSprite frame {\nend note\nCedar -> Spruce: go\n", True, ["multiline note text", "pixel header lookalike", mode], SB)
    add(f"cl_seq_note_brace_lower_{mode}", pre + "participant Oak\nparticipant Elm\nnote right of Elm\nsprite atlas {\nend note\nOak -> Elm: go\n", True, ["multiline note text", "pixel header lookalike lowercase", mode], SB)
    add(f"cl_seq_legend_upper_{mode}", pre + "participant Cedar\nparticipant Spruce\nCedar -> Spruce: go\nlegend\nSprite atlas version two\nendlegend\n", True, ["legend text", "Sprite capitalised", mode], SB)
    add(f"cl_seq_ref_upper_{mode}", pre + "participant Cedar\nparticipant Spruce\nref over Cedar, Spruce\nSprite cache warm\nend ref\n", True, ["ref text", "Sprite capitalised", mode], SB)
    add(f"cl_seq_lower_participant_msg_{mode}", pre + "sprite -> bob : hi\n", True, ["participant named sprite", "lowercase message", mode], SB)
    add(f"cl_seq_lower_participant_dashed_{mode}", pre + "participant Cedar\nsprite --> Cedar : reply\n", True, ["participant named sprite", "dashed message", mode], SB)
    add(f"cl_seq_inline_svg_apostrophe_{mode}", pre + "participant Cedar\nparticipant Spruce\nsprite $tag <svg width=\"4\" height=\"4\"><text> 'hi'</text></svg>\nCedar -> Spruce: go\n", True, ["inline svg", "space-apostrophe inside svg", mode], SB)
    add(f"cl_seq_inline_svg_apostrophe_attr_{mode}", pre + "participant Pine\nparticipant Fir\nsprite $mark <svg viewBox=\"0 0 4 4\"><title> 'mark'</title><rect width=\"2\" height=\"2\"/></svg>\nFir -> Pine: back\nPine -> Fir: forth\n", True, ["inline svg", "space-apostrophe inside title", mode], SB)
    add(f"cl_seq_inline_svg_trailing_comment_{mode}", pre + "participant Cedar\nsprite $tag <svg width=\"4\" height=\"4\"><rect width=\"1\" height=\"1\"/></svg> ' trailing\nCedar -> Cedar: go\n", False, ["inline svg", "text after closing svg", mode], SB)
    add(f"cl_seq_bare_sprite_{mode}", pre + "participant Cedar\nsprite foo\nCedar -> Cedar: go\n", False, ["sprite without value", "word name", mode], SB)
    add(f"cl_seq_bare_dollar_sprite_{mode}", pre + "participant Cedar\nsprite $x\nCedar -> Cedar: go\n", False, ["sprite without value", "dollar name", mode], SB)

for fam, decl in [("class", "class Player\n"), ("object", "object Player\n")]:
    add(f"cl_{fam}_link_upper", decl + "Sprite -- Player\n", True, ["class link", "Sprite capitalised", fam], SB)
    add(f"cl_{fam}_link_lower", decl + "sprite -- Player\n", True, ["class link", "sprite lowercase control", fam], SB)
add("cl_class_aggregation_label", "class Player\nSprite o-- Player : owns\n", True, ["class link", "aggregation with label"], SB)
add("cl_class_dotted", "class Player\nclass Level\nSprite .. Player\nLevel -- Player\n", True, ["class link", "dotted"], SB)
add("cl_class_member_upper", "class Player {\n  Sprite sheet field\n}\n", True, ["class body", "Sprite capitalised"], SB)
add("cl_class_member_brace", "class Player {\n  Sprite frame {\n}\nclass Level\n", True, ["class body", "pixel header lookalike"], SB)
add("cl_class_note_upper", "class Player\nnote right of Player\nSprite pool size\nend note\n", True, ["class note text", "Sprite capitalised"], SB)
add("cl_class_bare_sprite", "class Player\nsprite foo\n", False, ["sprite without value", "class"], SB)
add("cl_state_note_upper", "state Idle\nnote right of Idle\nSprite idle loop\nend note\n", True, ["state note text", "Sprite capitalised"], SB)
add("cl_activity_note_upper", "start\n:load;\nnote right\nSprite batch ready now\nend note\nstop\n", True, ["activity note text", "Sprite capitalised"], SB)
add("cl_component_link_upper", "[Engine]\nSprite -- [Engine]\n", False, ["description link", "sprite file command precedes link"], SB)
add("cl_usecase_link_upper", "actor Hero\nSprite -- (Play)\n", False, ["description link", "sprite file command precedes link"], SB)
add("cl_component_note_upper", "[Engine]\nnote right of [Engine]\nSprite cache budget\nend note\n", True, ["component note text", "Sprite capitalised"], SB)

manifest_cases, files = [], []
for name, src, valid, axes, mech in cases:
    assert not any(0xE000 <= ord(c) <= 0xE0FF or (ord(c) < 32 and c != "\n") for c in src), name
    p = INPUTS / f"{name}.puml"
    data = src.encode()
    p.write_bytes(data)
    h = hashlib.sha256(data).hexdigest()
    manifest_cases.append({"name": name, "source": str(p), "source_sha256": h, "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
        "invocation": ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"],
        "env": {}, "intended_valid": valid, "axes": axes, "mechanism": mech})
    files.append({"path": f"{name}.puml", "sha256": h, "base64": base64.b64encode(data).decode()})
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
    "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": manifest_cases}
mbytes = json.dumps(manifest, indent=1, ensure_ascii=False).encode()
(BATCH / "reference-manifest.json").write_bytes(mbytes)
bundle = {"manifest": manifest, "manifest_sha256": hashlib.sha256(mbytes).hexdigest(), "files": files, "resources": []}
with gzip.GzipFile(BATCH / "reference-input-bundle.json.gz", "wb", mtime=0) as fh:
    fh.write(json.dumps(bundle, ensure_ascii=False).encode())
print(len(cases), sum(c[2] for c in cases))
