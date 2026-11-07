"""Load a census script version, stub only the JVM/JDK constants, run record(OUT), log every audit write event."""
import importlib.util, json, os, sys
from pathlib import Path
script, out, log, jdk, ref, scratch = sys.argv[1:7]
events = []
WRITE = ("open", "os.mkdir", "os.rename", "os.replace", "os.remove", "os.rmdir", "os.symlink", "os.link", "os.chmod", "os.chown", "os.truncate", "os.utime", "shutil.copyfile", "shutil.rmtree", "subprocess.Popen", "os.posix_spawn", "os.exec")
def hook(event, args):
    if event == "open":
        path, mode, flags = args
        if isinstance(path, int):
            return
        if (mode and any(c in str(mode) for c in "wax+")) or (flags and flags & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND)):
            events.append(["write-open", str(path)])
    elif event in WRITE:
        events.append([event, repr(args)[:300]])
spec = importlib.util.spec_from_file_location("census", script)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
m.JDK_HOME = jdk
m.REFERENCE = ref
m.SCRATCH = Path(scratch)
sys.addaudithook(hook)
status = "ok"
try:
    m.record(Path(out))
except SystemExit as exc:
    status = f"SystemExit {exc}"
Path(log).write_text(json.dumps({"status": status, "events": events}, indent=1))
