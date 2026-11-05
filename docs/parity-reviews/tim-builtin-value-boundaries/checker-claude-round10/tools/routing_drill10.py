"""Round-10 routing drill: each mutant drops exactly one italic slot at one call site; run the lib gate filter.

Usage: python3 -I routing_drill10.py <private tree copy> <logdir> [<test filter>]
Each mutant is applied to a fresh copy of the listed file, the test run, and the file restored.
Expected: every mutant fails 'every_italic_slot_measures_in_its_own_face' except the Menlo bold-italic
face-call mutant (equivalent: Menlo Bold Italic and Menlo Bold hide the same scalars; EQUIVALENT list).
"""
import json, os, pathlib, subprocess, sys
T, LOG = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]); LOG.mkdir(parents=True, exist_ok=True)
FILTER = sys.argv[3] if len(sys.argv) > 3 else "text_render::tests::every_italic_slot_measures_in_its_own_face"
TR, PM = "crates/rustuml-render/src/text_render.rs", "crates/rustuml-render/src/plantuml_metrics.rs"
def face(fam):
    return f"ChainFace::for_style(ChainFamily::{fam}, bold, italic)"
M = {}
for fam, path in [("Arial", TR), ("Helvetica", TR), ("Verdana", TR), ("Menlo", PM), ("CourierNew", PM)]:
    M[f"{fam}_face_italic_slot_to_plain"] = (path, face(fam), f"ChainFace::for_style(ChainFamily::{fam}, bold, italic && bold)")
    M[f"{fam}_face_bolditalic_slot_to_bold"] = (path, face(fam), f"ChainFace::for_style(ChainFamily::{fam}, bold, italic && !bold)")
M["M15_checker_arial_italic_routed_plain"] = (TR, face("Arial"), "ChainFace::for_style(ChainFamily::Arial, bold, false)")
M["Mono_arm_italic_dropped"] = (TR, "MetricFamily::Mono => pm::mono_text_width_styled(text, font_size, bold, italic),",
                                "MetricFamily::Mono => pm::mono_text_width_styled(text, font_size, bold, false),")
M["CourierNew_arm_italic_dropped"] = (TR, "MetricFamily::CourierNew => pm::courier_new_text_width(text, font_size, bold, italic),",
                                      "MetricFamily::CourierNew => pm::courier_new_text_width(text, font_size, bold, false),")
M["Helvetica_arm_italic_dropped"] = (TR, "MetricFamily::Helvetica => helvetica_text_width(text, font_size, bold, italic),",
                                     "MetricFamily::Helvetica => helvetica_text_width(text, font_size, bold, false),")
env = dict(os.environ, CARGO_TARGET_DIR=str(T.parent / "target-drill10"))
def run(name):
    p = subprocess.run(["cargo", "test", "-j", "6", "-p", "rustuml-render", "--lib", FILTER], cwd=T, env=env,
                       capture_output=True, text=True)
    (LOG / f"{name}.log").write_text(p.stdout + p.stderr)
    return p.returncode, ("test result: ok. 1 passed" in p.stdout)
res = {}
rc, ok = run("M00_unmodified"); res["M00_unmodified"] = dict(exit=rc, passed=ok)
for name, (path, old, new) in M.items():
    f = T / path; orig = f.read_text()
    assert orig.count(old) == 1, (name, orig.count(old))
    f.write_text(orig.replace(old, new))
    try:
        rc, ok = run(name)
    finally:
        f.write_text(orig)
    res[name] = dict(file=path, old=old, new=new, exit=rc, passed=ok, caught=not ok)
    print(name, "caught" if not ok else "SURVIVED", flush=True)
(LOG / "drill.json").write_text(json.dumps(res, indent=1))
print(json.dumps({k: ("pass" if v["passed"] else "caught") for k, v in res.items()}, indent=0))
