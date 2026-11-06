"""Build the T14.47 variants (base and tip, v1/v2) and the consumer-drill mutant CLIs, one shared target per code base,
copy each binary to bin/, then delete the targets. Usage: python3 -I build_variants10.py"""
import json, os, pathlib, shutil, subprocess, sys
C = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round10-claude-checker"
spec = json.load(open(C / "drill/logs-consumer/consumer-drill.json"))
def build(tree, target, name):
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    p = subprocess.run(["cargo", "build", "--release", "-j", "6", "-p", "rustuml"], cwd=tree, env=env, capture_output=True, text=True)
    assert p.returncode == 0, (name, p.stderr[-2000:])
    shutil.copy(target / "release/rustuml", C / f"bin/rustuml-{name}"); print("built", name, flush=True)
for tree in ["95291dc6b", "e24240b35"]:
    tgt = C / f"target/variants-{tree}"
    for v in ["v1", "v2"]: build(C / f"t1447/{tree}-{v}", tgt, f"{tree}-{v}")
    if tree == "e24240b35":
        T = C / "drill/tree"
        for name, m in spec.items():
            if not name.startswith("C"): continue
            f = T / m["file"]; orig = f.read_text(); assert orig.count(m["old"]) == 1
            f.write_text(orig.replace(m["old"], m["new"]))
            try: build(T, C / "target/variants-drill", f"mut-{name}")
            finally: f.write_text(orig)
        shutil.rmtree(C / "target/variants-drill", ignore_errors=True)
    shutil.rmtree(tgt, ignore_errors=True)
print("VARIANTS DONE")
