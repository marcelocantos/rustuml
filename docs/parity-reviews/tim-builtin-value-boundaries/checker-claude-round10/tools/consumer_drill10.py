"""Checker round-10 consumer drill: drop the italic at one consumer measurement site (or the Times arm) and run the
whole rustuml-render lib suite; report which mutants any lib test catches.
Usage: python3 -I consumer_drill10.py <tree> <logdir>"""
import json, os, pathlib, re, subprocess, sys
T, LOG = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]); LOG.mkdir(parents=True, exist_ok=True)
S = "crates/rustuml-render/src/"
M = {
 "T1_times_arm_italic_dropped": (S + "text_render.rs", "pm::times_new_roman_text_width(text, font_size, bold, italic)", "pm::times_new_roman_text_width(text, font_size, bold, false)", 1),
 "C1_component_label_measure_italic_dropped": (S + "component.rs", "        label_bold,\n        label_italic,\n        font_family,\n    );", "        label_bold,\n        false,\n        font_family,\n    );", 1),
 "C2_component_stereotype_measured_plain": (S + "component.rs", "                STEREOTYPE_FONT_BOLD,\n                STEREOTYPE_FONT_ITALIC,\n                font_family,", "                STEREOTYPE_FONT_BOLD,\n                false,\n                font_family,", 1),
 "C3_participant_block_italic_dropped_5663": (S + "sequence.rs", "                        !stereotype && self.participant_font_bold,\n                        *stereotype || self.participant_font_italic,\n                    ),", "                        !stereotype && self.participant_font_bold,\n                        false,\n                    ),", 1),
 "C4_activity_layout_italic_dropped": (S + "activity.rs", "        palette.action_text_bold,\n        palette.action_text_italic,\n        &palette.action_font_family,\n    );\n    LayoutNode::Action {", "        palette.action_text_bold,\n        false,\n        &palette.action_font_family,\n    );\n    LayoutNode::Action {", 1),
 "C5_state_node_font_italic_dropped": (S + "state.rs", "        italic: state_font_italic,", "        italic: false,", 1),
 "C6_usecase_label_measure_italic_dropped": (S + "usecase.rs", "        &uc.label,\n        font_size,\n        skin.uc_font_bold,\n        skin.uc_font_italic,", "        &uc.label,\n        font_size,\n        skin.uc_font_bold,\n        false,", 1),
}
env = dict(os.environ, CARGO_TARGET_DIR=str(T.parent / "target-cdrill10"), PLANTUML_STDLIB=str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib"))
def run(name):
    p = subprocess.run(["cargo", "test", "--release", "-j", "6", "-p", "rustuml-render", "--lib", "--no-fail-fast"], cwd=T, env=env, capture_output=True, text=True)
    (LOG / f"{name}.log").write_text(p.stdout + p.stderr)
    fails = re.findall(r"^    (\S+)$", p.stdout, re.M)
    m = re.search(r"test result: \w+\. (\d+) passed; (\d+) failed", p.stdout)
    return p.returncode, (m.groups() if m else None), fails
res = {}
rc, counts, fails = run("M00_unmodified"); res["M00_unmodified"] = dict(exit=rc, counts=counts, failed=fails)
for name, (path, old, new, n) in M.items():
    f = T / path; orig = f.read_text(); assert orig.count(old) == n, (name, orig.count(old))
    f.write_text(orig.replace(old, new))
    try: rc, counts, fails = run(name)
    finally: f.write_text(orig)
    base = set(res["M00_unmodified"]["failed"])
    res[name] = dict(file=path, old=old, new=new, exit=rc, counts=counts, new_failures=sorted(set(fails) - base), caught=bool(set(fails) - base))
    print(name, "caught" if res[name]["caught"] else "SURVIVED", res[name]["new_failures"][:5], flush=True)
(LOG / "consumer-drill.json").write_text(json.dumps(res, indent=1))
