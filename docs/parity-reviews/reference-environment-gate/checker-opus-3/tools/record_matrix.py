"""Record-level matrix: real record() with only the JVM stubbed and font roots moved into a sandbox.

Writes are captured with a Python audit hook (open for write, mkdir, rename, remove, symlink, chmod).
Each record is a separate subprocess so a crash is an exit code, as in the hook.
"""
import json, os, shutil, subprocess, sys
from pathlib import Path

script, jdk, stub, base = sys.argv[1], sys.argv[2], sys.argv[3], Path(sys.argv[4]).resolve()
base.mkdir(parents=True)
CHILD = r'''
import importlib.util, json, os, sys
from pathlib import Path
script, jdk, stub, scratch, fonts, out, log = sys.argv[1:8]
events = []
def hook(event, args):
    if event == "open":
        path, mode, flags = args
        if isinstance(path, int): return
        if (mode and any(c in str(mode) for c in "wax+")) or (flags and flags & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND)):
            events.append(["write", os.path.abspath(str(path))])
    elif event in ("os.mkdir", "os.rename", "os.replace", "os.remove", "os.rmdir", "os.symlink", "os.link", "os.chmod", "os.truncate"):
        events.append([event, os.path.abspath(str(args[0]))])
spec = importlib.util.spec_from_file_location("census", script)
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
m.JDK_HOME = jdk; m.REFERENCE = stub
fonts = Path(fonts)
m.FONT_FILES = (str(fonts / "system/Modelled.ttc"),)
m.CASCADE_FONT_FILES = (str(fonts / "system/Cascade A.ttc"), str(fonts / "system/Cascade B.ttf"))
m.USER_FONT_DIRS = (fonts / "user",)
m.ASSET_FONT_DIRS = (fonts / "assets",)
m.prepare_record_dir.__defaults__ = (Path(scratch),)
sys.addaudithook(hook)
code = 0
try:
    m.record(Path(out))
except SystemExit as exc:
    code = 1; events.append(["SystemExit", str(exc)])
except Exception as exc:
    code = 1; events.append([type(exc).__name__, str(exc)])
Path(log).write_text(json.dumps(events))
sys.exit(code)
'''
child = base / "child.py"; child.write_text(CHILD)
scratch = base / "scratch"; scratch.mkdir()
fonts = base / "fonts"
def reset_fonts():
    if fonts.exists(): shutil.rmtree(fonts)
    for d in ("system", "user", "assets/a.asset/AssetData", "elsewhere"): (fonts / d).mkdir(parents=True)
    (fonts / "system/Modelled.ttc").write_bytes(b"modelled")
    (fonts / "system/Cascade A.ttc").write_bytes(b"cascade-a")
    (fonts / "system/Cascade B.ttf").write_bytes(b"cascade-b")
    (fonts / "user/User.ttf").write_bytes(b"user")
    (fonts / "assets/a.asset/AssetData/Asset.ttc").write_bytes(b"asset")
n = [0]
def record(out, env_extra=None):
    n[0] += 1
    log = base / f"log-{n[0]}.json"
    env = dict(os.environ, **(env_extra or {}))
    p = subprocess.run([sys.executable, "-I", str(child), script, jdk, stub, str(scratch), str(fonts), str(out), str(log)], capture_output=True, text=True, env=env)
    return p.returncode, json.loads(log.read_text())
def check(live, committed):
    p = subprocess.run([sys.executable, "-I", script, "check", str(live), "--committed", str(committed)], capture_output=True, text=True)
    return p.returncode, [l for l in p.stdout.splitlines() if l.startswith(("DRIFT", "UNPINNED"))]
def outside(events, out):
    real = os.path.realpath(out)
    bad = []
    for ev, path in events:
        if ev in ("SystemExit",) or ev[0].isupper(): continue
        rp = os.path.realpath(path)
        if not (rp == real or rp.startswith(real + os.sep)):
            bad.append([ev, path, rp])
    return bad
print("record_exit\tcheck_exit\texpected\tstatus\tcase\tdetail")
def row(case, rexit, cexit, expected, detail=""):
    got = f"{rexit}/{cexit}"
    print(f"{rexit}\t{cexit}\t{expected}\t{'ok' if got == expected else 'UNEXPECTED'}\t{case}\t{detail}")

reset_fonts()
baseline = scratch / "baseline"
rexit, ev = record(baseline)
row("baseline-record", rexit, "-", "0/-", f"outside={outside(ev, baseline)} files={sorted(p.name for p in baseline.iterdir())}")
committed = base / "baseline-census.txt"; shutil.copy(baseline / "census.txt", committed)
def drift_case(name, mutate, expected):
    reset_fonts(); mutate()
    out = scratch / name
    rexit, ev = record(out)
    if rexit != 0:
        row(name, rexit, "-", expected, (ev[-1][1] if ev else "")[:140]); return
    cexit, lines = check(out, committed)
    row(name, rexit, cexit, expected, "; ".join(lines)[:220] + f" outside={outside(ev, out)}")
drift_case("unchanged-host", lambda: None, "0/0")
drift_case("cascade-face-changed", lambda: (fonts / "system/Cascade A.ttc").write_bytes(b"cascade-A"), "0/1")
drift_case("cascade-face-removed", lambda: (fonts / "system/Cascade A.ttc").unlink(), "1/-")
def to_symlink(target_bytes, same_name=True):
    def f():
        src = fonts / "system/Cascade A.ttc"
        tgt = fonts / "elsewhere/real.ttc"; tgt.write_bytes(target_bytes)
        src.unlink(); src.symlink_to(tgt)
    return f
drift_case("cascade-face-symlink-other-bytes", to_symlink(b"other"), "0/1")
drift_case("cascade-face-symlink-same-bytes", to_symlink(b"cascade-a"), "0/0")
def dangling():
    src = fonts / "system/Cascade A.ttc"; src.unlink(); src.symlink_to(fonts / "elsewhere/missing.ttc")
drift_case("cascade-face-dangling-symlink", dangling, "1/-")
drift_case("modelled-face-changed", lambda: (fonts / "system/Modelled.ttc").write_bytes(b"MODELLED"), "0/1")
drift_case("user-font-added", lambda: (fonts / "user/New.ttf").write_bytes(b"n"), "0/1")
def user_symlink():
    (fonts / "elsewhere/u.ttf").write_bytes(b"user"); (fonts / "user/User.ttf").unlink(); (fonts / "user/User.ttf").symlink_to(fonts / "elsewhere/u.ttf")
drift_case("user-font-became-symlink-same-bytes", user_symlink, "0/1")
drift_case("asset-font-changed", lambda: (fonts / "assets/a.asset/AssetData/Asset.ttc").write_bytes(b"ASSET"), "0/1")
drift_case("asset-catalog-added", lambda: ((fonts / "assets/catalog.xml").write_bytes(b"x"), (fonts / "assets/a.asset/Info.plist").write_bytes(b"p")), "0/0")
drift_case("jvm-exit-nonzero", lambda: os.environ.__setitem__("STUB_EXIT", "1"), "1/-")
os.environ.pop("STUB_EXIT", None)

# ---- OUT confinement ----
reset_fonts()
def out_case(name, out, expected_exit, watch):
    before = {str(p) for w in watch for p in Path(w).rglob("*")} if watch else set()
    rexit, ev = record(out)
    after = {str(p) for w in watch for p in Path(w).rglob("*")} if watch else set()
    created = sorted(after - before)
    status = "ok" if rexit == expected_exit else "UNEXPECTED"
    print(f"{rexit}\t-\t{expected_exit}/-\t{status}\t{name}\toutside_writes={outside(ev, out) if rexit == 0 else [e for e in ev if e[0] in ('write','os.mkdir')]} created_in_watched={created} reason={(ev[-1][1] if ev and ev[-1][0]=='SystemExit' else '')[:110]}")
outside_dir = base / "outside-target"; outside_dir.mkdir()
link_out = scratch / "link-out"; link_out.symlink_to(outside_dir, target_is_directory=True)
out_case("symlink-OUT-to-empty-dir", link_out, 1, [outside_dir])
dang = scratch / "dangling-out"; dang.symlink_to(base / "nowhere", target_is_directory=True)
out_case("symlink-OUT-dangling", dang, 1, [base])
real_parent = base / "real-parent"; real_parent.mkdir()
hop = scratch / "hop"; hop.symlink_to(real_parent, target_is_directory=True)
out_case("symlink-ancestor-under-scratch", hop / "out", 1, [real_parent])
deep = scratch / "d1"; deep.mkdir(); (deep / "hop2").symlink_to(real_parent, target_is_directory=True); (real_parent / "sub").mkdir()
out_case("symlink-ancestor-two-levels-under-scratch", deep / "hop2" / "sub" / "out", 1, [real_parent])
out_case("dotdot-into-scratch-symlink", scratch / "d1" / ".." / "hop" / "out2", 1, [real_parent])
plain_outside = base / "plain-outside"; plain_outside.mkdir()
out_case("non-symlink-ancestor-outside-scratch", plain_outside / "out", 0, [plain_outside])
link_outside = base / "link-outside"; link_outside.symlink_to(real_parent, target_is_directory=True)
out_case("symlink-ancestor-outside-scratch (by design: allowed)", link_outside / "out3", 0, [real_parent])
alias = base / "alias-to-scratch"; alias.symlink_to(scratch, target_is_directory=True)
out_case("scratch-reached-through-outside-alias, then hop", alias / "hop" / "out4", 0, [real_parent])
sym_scratch_target = base / "scratch-target"; sym_scratch_target.mkdir()
sym_scratch = base / "sym-scratch"; sym_scratch.symlink_to(sym_scratch_target, target_is_directory=True)
# scratch itself symlinked: rebind the default to the symlinked scratch path
CHILD2 = CHILD.replace("m.prepare_record_dir.__defaults__ = (Path(scratch),)", "m.prepare_record_dir.__defaults__ = (Path(scratch),)")
p = subprocess.run([sys.executable, "-I", str(child), script, jdk, stub, str(sym_scratch), str(fonts), str(sym_scratch / "out"), str(base / "log-symscratch.json")], capture_output=True, text=True)
print(f"{p.returncode}\t-\t1/-\t{'ok' if p.returncode == 1 else 'UNEXPECTED'}\tscratch-itself-symlink\tcreated_in_target={sorted(str(x) for x in sym_scratch_target.rglob('*'))} reason={json.loads((base / 'log-symscratch.json').read_text())[-1][1][:110]}")
occupied = scratch / "occupied"; occupied.mkdir(); (occupied / "keep").write_text("x")
out_case("OUT-non-empty", occupied, 1, [occupied])
out_case("OUT-missing-parent", scratch / "absent" / "out", 1, [scratch])
print(f"-\t-\t-\t{'ok' if not (scratch / 'absent').exists() else 'UNEXPECTED'}\tmissing-parent-not-created\t")
regular = scratch / "regular-file"; regular.write_text("x")
out_case("OUT-is-regular-file", regular, 1, [scratch])
empty = scratch / "empty-existing"; empty.mkdir()
out_case("OUT-existing-empty-dir", empty, 0, [scratch])
