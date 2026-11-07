"""Stage maker batch2: held-out Java reference request for post-TIM sprite ownership and participant/colour residue.

Usage: python3 -I stage_batch2.py DEST   (DEST must not exist)"""
import base64, gzip, hashlib, json, os, sys
from pathlib import Path
dest = Path(sys.argv[1]); inputs = dest / "inputs"; inputs.mkdir(parents=True, exist_ok=False)
sha = lambda b: hashlib.sha256(b).hexdigest()
PIX = "sprite $gem [2x2/16] {\nF0\n0F\n}"
PIX_OPEN = "sprite $gem [2x2/16] {\nF0\n0F"
SVG_OPEN = 'sprite $gem <svg width="2" height="2">\n<rect width="2" height="2"/>'
SEQ = "participant Cedar\nparticipant Spruce\nCedar -> Spruce: compass\n"
TEOZ = "!pragma teoz true\n"
cases = []
def case(name, body, valid, axes, mechanism="sprite-command-post-tim-ownership"):
    cases.append((name, "@startuml\n" + body.rstrip("\n") + "\n@enduml\n", valid, axes, mechanism))
for mode, pre in [("legacy", ""), ("teoz", TEOZ)]:
    case(f"mk2_if_false_pixel_seq_{mode}", pre + SEQ + f"!if %false()\n{PIX_OPEN}\n!endif\nSpruce -> Cedar: back", True, ["!if false", "unclosed pixel", mode])
    case(f"mk2_else_branch_pixel_seq_{mode}", pre + SEQ + f"!if %false()\n{SVG_OPEN}\n!else\n{PIX}\n!endif\nSpruce -> Cedar: back", True, ["!if false then !else", "closed pixel", mode])
    case(f"mk2_proc_invoked_closed_pixel_seq_{mode}", pre + SEQ + f"!procedure $gem()\n{PIX}\n!endprocedure\n$gem()\nSpruce -> Cedar: back", True, ["invoked procedure", "closed pixel", mode])
    case(f"mk2_proc_invoked_unclosed_svg_seq_{mode}", pre + SEQ + f"!procedure $gem()\n{SVG_OPEN}\n!endprocedure\n$gem()\nSpruce -> Cedar: back", False, ["invoked procedure", "unclosed svg", mode])
    case(f"mk2_include_close_only_seq_{mode}", pre + SEQ + 'sprite $gem <svg width="2" height="2">\n!include mk2_close.inc\nSpruce -> Cedar: back', True, ["!include supplies body and END", "svg", mode])
    case(f"mk2_seq_note_svg_lookalike_{mode}", pre + SEQ + 'note over Cedar\nsprite $gem <svg width="2" height="2">\nend note\nSpruce -> Cedar: back', True, ["note body", "svg header lookalike", mode])
    case(f"mk2_seq_top_level_text_{mode}", pre + SEQ + "Sprite sheets are loaded", False, ["top-level Sprite text", "CommandSpriteFile", mode])
    case(f"mk2_redeclare_into_box_{mode}", pre + "participant Hazel\nparticipant Elm\nbox \"Copse\"\nparticipant Hazel\nend box\nHazel -> Elm: catkins", True, ["putParticipantInLast", "box englober", mode], "sequence-participant-silent-residue")
    case(f"mk2_multi_stereo_three_{mode}", pre + "participant Spruce\nparticipant Cedar<<woody>><<tall>><<old>>\nSpruce -> Cedar: pollen", True, ["three stereotypes", mode], "sequence-participant-silent-residue")
    case(f"mk2_tooltip_in_box_{mode}", pre + "box \"Grove\"\nparticipant Cedar [[https://example.org/c{bark} cedar]]\nend box\nparticipant Spruce\nSpruce -> Cedar: pollen", True, ["participant url tooltip", "inside box", mode], "sequence-participant-silent-residue")
# Colour-free controls for the colour probes whose fills already match.
case("mk2_note_across_plain_legacy", SEQ + "note across: rings", True, ["note across", "no colour control"], "sequence-participant-silent-residue")
case("mk2_hnote_over_plain_legacy", SEQ + "hnote over Cedar,Spruce: rings", True, ["hnote over several", "no colour control"], "sequence-participant-silent-residue")
case("mk2_rnote_across_multi_plain_legacy", SEQ + "rnote across\nrings\nendrnote", True, ["rnote across multiline", "no colour control"], "sequence-participant-silent-residue")
case("mk2_hnote_right_plain_teoz", TEOZ + SEQ + "hnote right: rings", True, ["hnote right", "no colour control", "teoz"], "sequence-participant-silent-residue")
case("mk2_box_plain_teoz", TEOZ + SEQ + "box \"Forest\"\nparticipant Birch\nend box", True, ["box", "no colour control", "teoz"], "sequence-participant-silent-residue")
case("mk2_group_space_colour_teoz", TEOZ + SEQ + "group #Indigo shade\nCedar -> Spruce: roots\nend", True, ["group COLORS[1] only", "teoz"], "sequence-participant-silent-residue")
case("mk2_return_hash_label_legacy", SEQ + "activate Spruce\nreturn #1 pollen", True, ["return label starting with #", "creole list"], "sequence-participant-silent-residue")
# Other families: TIM order, bodies, unclosed blocks, execution failures.
case("mk2_if_false_svg_class", "class Player\n!if %false()\n" + SVG_OPEN + "\n!endif\nclass Level\nPlayer -- Level", True, ["!if false", "unclosed svg", "class"])
case("mk2_if_true_closed_pixel_component", "!if %true()\n" + PIX + "\n!endif\n[Engine]\n[Wheel]\n[Engine] --> [Wheel]", True, ["!if true", "closed pixel", "component"])
case("mk2_if_true_unclosed_pixel_state", "state Idle\n!if %true()\n" + PIX_OPEN + "\n!endif\nIdle --> Busy", False, ["!if true", "unclosed pixel", "state"])
case("mk2_proc_uninvoked_svg_usecase", "actor Hero\n!procedure $gem()\n" + SVG_OPEN + "\n!endprocedure\nHero --> (Play)", True, ["uninvoked procedure", "unclosed svg", "usecase"])
case("mk2_include_header_only_class", "class Player\n!include mk2_hdr.inc\n}\nclass Level\nPlayer -- Level", True, ["!include supplies header and rows", "pixel", "class"])
case("mk2_include_unclosed_component", "[Engine]\n!include mk2_open_pixel.inc\n[Wheel]", False, ["!include opens pixel never closed", "component"])
case("mk2_state_note_brace", "state Idle\nnote right of Idle\nSprite frame {\nend note\nIdle --> Busy", True, ["state note body", "pixel header lookalike"])
case("mk2_activity_note_svg", "start\n:load;\nnote right\nsprite $gem <svg width=\"2\" height=\"2\">\nend note\nstop", True, ["activity note body", "svg header lookalike"])
case("mk2_usecase_note_upper", "actor Hero\nnote right of Hero\nSprite pool size\nend note\nHero --> (Play)", True, ["usecase note body", "Sprite capitalised"])
case("mk2_deployment_note_brace", "node Server\nnote right of Server\nSprite frame {\nend note\nnode Client\nClient --> Server", True, ["deployment note body", "pixel header lookalike"])
case("mk2_component_legend_upper", "[Engine]\nlegend\nSprite legend line\nendlegend", True, ["legend body", "component"])
case("mk2_class_member_svg", "class Player {\n  sprite $gem <svg width=\"2\" height=\"2\">\n}\nclass Level\nPlayer -- Level", True, ["class body", "svg header lookalike"])
case("mk2_class_top_level_text", "class Player\nSprite sheets are loaded", False, ["top-level Sprite text", "class", "CommandSpriteFile"])
case("mk2_component_top_level_text", "[Engine]\nSprite sheets are loaded", False, ["top-level Sprite text", "component", "CommandSpriteFile"])
for fam, head in [("object", "object Player"), ("deployment", "node Server"), ("activity_legacy", "(*) --> \"Load\""), ("timing", "robust \"Web\" as WB\n@0\nWB is Idle"), ("mindmap_body", None)]:
    if head is None:
        continue
    case(f"mk2_unclosed_pixel_{fam}", head + "\n" + PIX_OPEN, False, ["unclosed pixel", fam])
    case(f"mk2_unclosed_svg_{fam}", head + "\n" + SVG_OPEN, False, ["unclosed svg", fam])
cases.append(("mk2_unclosed_svg_mindmap", "@startmindmap\n* Root\n" + SVG_OPEN + "\n** Leaf\n@endmindmap\n", False, ["unclosed svg", "mindmap"], "sprite-command-post-tim-ownership"))
case("mk2_empty_pixel_class", "class Player\nsprite $gem {\n}", False, ["empty pixel body", "No sprite defined.", "class"])
case("mk2_graylevel_component", "[Engine]\nsprite $gem [2x2/7] {\nF0\n0F\n}", False, ["graylevel 7", "component"])
case("mk2_sprite_file_missing_deployment", "node Server\nsprite $gem missing_gem.png", False, ["CommandSpriteFile", "missing file", "deployment"])
case("mk2_compressed_bad_class", "class Player\nsprite $gem [3x2/16z] 0000", False, ["z-compressed", "Cannot decode sprite.", "class"])
resources = {
    "mk2_close.inc": '<rect width="2" height="2"/>\n</svg>\n',
    "mk2_hdr.inc": "sprite $gem [2x2/16] {\nF0\n0F\n",
    "mk2_open_pixel.inc": "sprite $gem [2x2/16] {\nF0\n",
}
INVOCATION = ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"]
rows, files = [], []
for name, text, valid, axes, mechanism in cases:
    data = text.encode()
    assert not any(0xE000 <= ord(c) <= 0xE0FF for c in text), name
    assert not any(ord(c) < 0x20 and c != "\n" for c in text), name
    path = inputs / f"{name}.puml"; path.write_bytes(data); os.chmod(path, 0o444)
    rows.append(dict(name=name, source=str(path), source_sha256=sha(data), cwd="PRIVATE_CWD", timezone="GMT+11:00",
                     invocation=INVOCATION, env={}, intended_valid=valid, axes=axes, mechanism=mechanism))
    files.append(dict(path=f"{name}.puml", sha256=sha(data), base64=base64.b64encode(data).decode()))
res = []
for rpath, text in resources.items():
    data = text.encode(); p = inputs / rpath; p.write_bytes(data); os.chmod(p, 0o444)
    res.append(dict(path=rpath, sha256=sha(data), base64=base64.b64encode(data).decode()))
manifest = dict(schema_version=1, java_revision="71806a23780b04a5ccde2f8ceb5121edad5eb711",
                jar_sha256="aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", cases=rows)
mbytes = json.dumps(manifest, indent=1).encode()
(dest / "reference-manifest.json").write_bytes(mbytes)
bundle = dict(manifest=manifest, manifest_sha256=sha(mbytes), files=files, resources=res)
with gzip.GzipFile(dest / "reference-input-bundle.json.gz", "wb", mtime=0) as f:
    f.write(json.dumps(bundle).encode())
for p in (dest / "reference-manifest.json", dest / "reference-input-bundle.json.gz"):
    os.chmod(p, 0o444)
print(len(rows), "cases", sum(r["intended_valid"] for r in rows), "intended valid;", len(res), "resources")
