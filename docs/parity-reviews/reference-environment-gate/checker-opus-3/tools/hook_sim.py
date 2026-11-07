"""Simulate scripts/reference_env_gate.py against a fake install, and the documented seq_capture.py paste.

Only path constants are rewritten in copies: INSTALL/SCRATCH/PYTHON in the gate; REFERENCE/JDK_HOME/SCRATCH
in the census script; ROOT/JAR and the gate path in seq_capture.py. Code is otherwise byte-for-byte.
"""
import base64, gzip, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path
repo, jdk, stub, seq_src, base = Path(sys.argv[1]), sys.argv[2], sys.argv[3], Path(sys.argv[4]), Path(sys.argv[5]).resolve()
base.mkdir(parents=True)
PY = "/opt/homebrew/bin/python3"
def sub(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)
def make_install(name, scratch):
    inst = base / name / "reference-env"; inst.mkdir(parents=True)
    s = (repo / "scripts/reference_env_census.py").read_text(encoding="utf-8")
    s = sub(s, 'REFERENCE = "/Users/marcelo/rustuml-reference/bin/reference"', f'REFERENCE = "{stub}"')
    s = sub(s, 'JDK_HOME = "/opt/homebrew/Cellar/openjdk@21/21.0.5/libexec/openjdk.jdk/Contents/Home"', f'JDK_HOME = "{jdk}"')
    s = sub(s, 'SCRATCH = Path("/Users/marcelo/rustuml-reference/scratch")', f'SCRATCH = Path("{scratch}")')
    (inst / "reference_env_census.py").write_text(s, encoding="utf-8")
    g = (repo / "scripts/reference_env_gate.py").read_text(encoding="utf-8")
    g = sub(g, 'INSTALL = Path("/Users/marcelo/rustuml-reference/reference-env")', f'INSTALL = Path("{inst}")')
    g = sub(g, 'SCRATCH = Path("/Users/marcelo/rustuml-reference/scratch")', f'SCRATCH = Path("{scratch}")')
    (inst / "reference_env_gate.py").write_text(g, encoding="utf-8")
    return inst
def snapshot(root):
    return {str(p): (p.is_symlink(), p.stat().st_mtime_ns if p.exists() and not p.is_symlink() else 0) for p in Path(root).rglob("*")}
def gate(inst, env_extra=None):
    env = dict(os.environ, **(env_extra or {}))
    code = "import importlib.util,sys\nfrom pathlib import Path\n_s=importlib.util.spec_from_file_location('g',Path(sys.argv[1]));_g=importlib.util.module_from_spec(_s);_s.loader.exec_module(_g)\ntry:\n    _g.require_reference_env()\n    print('RETURNED')\nexcept SystemExit as e:\n    print('SYSTEMEXIT', repr(str(e))); raise\n"
    p = subprocess.run([PY, "-I", "-c", code, str(inst / "reference_env_gate.py")], capture_output=True, text=True, env=env)
    return p.returncode, p.stdout.strip().splitlines()[-1] if p.stdout.strip() else "", p.stderr.strip().splitlines()[-1] if p.stderr.strip() else ""
print("== hook ==")
scratch = base / "mm" / "scratch"; scratch.mkdir(parents=True)
inst = make_install("mm", scratch)
# The committed census for this fake host is a record of this host by the same installed script.
seed = base / "mm" / "seed"; 
r = subprocess.run([PY, "-I", str(inst / "reference_env_census.py"), "record", str(seed)], capture_output=True, text=True); assert r.returncode == 0, r.stderr
good = (seed / "census.txt").read_text(encoding="utf-8"); shutil.rmtree(seed)
def case(name, census_text, env_extra=None, expect=1):
    (inst / "census.txt").write_text(census_text, encoding="utf-8")
    outside_before = snapshot(base / "mm"); 
    rc, out, err = gate(inst, env_extra)
    after = snapshot(base / "mm")
    changed = sorted(k for k in set(after) ^ set(outside_before)) + sorted(k for k in set(after) & set(outside_before) if after[k] != outside_before[k] and "/scratch" not in k)
    temps = sorted(p.name for p in scratch.iterdir()) if scratch.is_dir() and not scratch.is_symlink() else []
    print(f"{rc}\t{expect}\t{'ok' if rc == expect else 'UNEXPECTED'}\t{name}\t{out}\ttemps_left={temps}\tfs_changes={changed}")
case("both-exit-0", good, expect=0)
case("check-exit-1 (harfbuzz hash moved)", good.replace(good.split("shaping.harfbuzz.sha256=")[1].split("\n")[0], "0" * 64))
case("check-exit-2 (scalar dash)", good.replace("host.product_version=27.2", "host.product_version=-"))
case("check-exit-2 (scalar blank both sides impossible live; committed blank)", good.replace("host.build_version=26B123", "host.build_version="))
case("record-exit-1 (JVM stub exits 1)", good, {"STUB_EXIT": "1"})
fake_font = None
case("check-exit-1 (committed has extra font row)", good.rstrip("\n") + "\nfont\t" + "ab" * 32 + "\t/System/Library/Fonts/NotThere.ttc\n")
(inst / "census.txt").unlink(); rc, out, err = gate(inst); print(f"{rc}\t1\t{'ok' if rc==1 else 'UNEXPECTED'}\tmissing-installed-census\t{out}")
(inst / "census.txt").write_text(good, encoding="utf-8")
# missing scratch
shutil.rmtree(scratch); rc, out, err = gate(inst); print(f"{rc}\t1\t{'ok' if rc==1 else 'UNEXPECTED'}\tmissing-scratch\t{out}\tcreated={scratch.exists()}")
# symlinked scratch
target = base / "mm" / "scratch-target"; target.mkdir(); scratch.symlink_to(target, target_is_directory=True)
rc, out, err = gate(inst); print(f"{rc}\t1\t{'ok' if rc==1 else 'UNEXPECTED'}\tsymlinked-scratch\t{out}\tleft_in_target={sorted(p.name for p in target.iterdir())}")
scratch.unlink(); scratch.mkdir()
# scratch is a regular file
shutil.rmtree(scratch); scratch.write_text("x"); rc, out, err = gate(inst); print(f"{rc}\t1\t{'ok' if rc==1 else 'UNEXPECTED'}\tscratch-is-file\t{out}"); scratch.unlink(); scratch.mkdir()

print("== documented seq_capture.py paste ==")
root = base / "relay-root"; (root / "bin").mkdir(parents=True)
jar = b"fake jar"; (root / "reference.jar").write_bytes(jar)
sentinel = base / "new-worker-ran"
(root / "bin" / "new-worker").write_text(f"#!/bin/sh\nmkdir -p {base}/worker\ntouch {sentinel}\necho {base}/worker\n"); os.chmod(root / "bin" / "new-worker", 0o755)
(root / "bin" / "reference").write_text("#!/bin/sh\nexit 0\n"); os.chmod(root / "bin" / "reference", 0o755)
src = seq_src.read_text(encoding="utf-8")
gate_doc = (repo / "scripts/reference_env_gate.py").read_text(encoding="utf-8")
block = gate_doc.split("Call site, inserted in ``seq_capture.py`` immediately before ``new-worker``::\n\n")[1].split("\n\nWhen the census")[0]
snippet = "\n".join(l for l in block.splitlines() if "new-worker" not in l).rstrip() + "\n"
anchor = "    worker = Path(subprocess.check_output([str(ROOT / 'bin/new-worker')], text=True).strip())\n"
assert src.count(anchor) == 1
pasted = src.replace(anchor, snippet + "\n" + anchor)
pasted_real = pasted
pasted = sub(pasted, "ROOT = Path('/Users/marcelo/rustuml-reference')", f"ROOT = Path('{root}')")
pasted = sub(pasted, "JAR = 'aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'", f"JAR = '{hashlib.sha256(jar).hexdigest()}'")
pasted = sub(pasted, "Path('/Users/marcelo/rustuml-reference/reference-env/reference_env_gate.py')", f"Path('{inst / 'reference_env_gate.py'}')")
(base / "seq_capture.pasted-real.py").write_text(pasted_real, encoding="utf-8")
(base / "seq_capture.sim.py").write_text(pasted, encoding="utf-8")
print("pasted snippet:"); print(snippet)
puml = b"@startuml\nA -> B\n@enduml\n"
manifest = json.dumps({"cases": [{"name": "c1", "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00", "invocation": [str(root / "bin/reference"), "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"], "source_sha256": hashlib.sha256(puml).hexdigest()}]}).encode()
payload = {"original_input_bundle_sha256": "o" * 64, "manifest_base64": base64.b64encode(manifest).decode(), "manifest_sha256": hashlib.sha256(manifest).hexdigest(), "manifest": json.loads(manifest), "files": [{"path": "c1.puml", "base64": base64.b64encode(puml).decode(), "sha256": hashlib.sha256(puml).hexdigest()}]}
z = gzip.compress(json.dumps(payload).encode()); inp = base / "input.json.gz"; inp.write_bytes(z)
args = [str(inp), hashlib.sha256(z).hexdigest(), "o" * 64, hashlib.sha256(manifest).hexdigest(), "1", "sim"]
for name, census_text, env_extra, expect_sentinel in (("gate-exit-1", good.replace("host.product_version=27.2", "host.product_version=27.3"), {}, False), ("gate-exit-2", good.replace("host.product_version=27.2", "host.product_version=-"), {}, False), ("gate-record-1", good, {"STUB_EXIT": "1"}, False), ("gate-pass", good, {}, True)):
    (inst / "census.txt").write_text(census_text, encoding="utf-8")
    if sentinel.exists(): sentinel.unlink()
    p = subprocess.run([PY, str(base / "seq_capture.sim.py"), *args], capture_output=True, text=True, env=dict(os.environ, **env_extra))
    ran = sentinel.exists()
    print(f"{p.returncode}\t{name}\tnew_worker_ran={ran}\t{'ok' if ran == expect_sentinel else 'UNEXPECTED'}\t{(p.stderr.strip().splitlines() or [''])[-1][:150]}")
