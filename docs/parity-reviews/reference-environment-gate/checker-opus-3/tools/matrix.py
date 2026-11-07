"""Independent check() mutation matrix against the committed census4 text.

Each case writes LIVE/census.txt and a committed file under ROOT/<case>/ and runs
the committed script's ``check`` as a subprocess (python3 -I), recording its exit code.
"""
import subprocess, sys, itertools
from pathlib import Path

script, committed_path, root = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
root.mkdir(parents=True)
T = committed_path.read_text(encoding="utf-8")
lines = T.splitlines()

def run(name, live, committed, expect):
    d = root / name
    (d / "live").mkdir(parents=True)
    (d / "live" / "census.txt").write_text(live, encoding="utf-8")
    (d / "committed.txt").write_text(committed, encoding="utf-8")
    p = subprocess.run([sys.executable, "-I", str(script), "check", str(d / "live"), "--committed", str(d / "committed.txt")], capture_output=True, text=True)
    status = "ok" if p.returncode == expect else "UNEXPECTED"
    print(f"{p.returncode}\t{expect}\t{status}\t{name}")
    return p.returncode

def once(text, old, new):
    assert text.count(old) == 1, (old, text.count(old))
    return text.replace(old, new, 1)

def line_starting(prefix, pred=lambda l: True):
    return next(l for l in lines if l.startswith(prefix) and pred(l))

def set_scalar(text, key, value):
    old = line_starting(key + "=")
    if value is None:
        return text.replace(old + "\n", "", 1)
    return once(text, old, f"{key}={value}")

def drop(text, line):
    assert text.count(line + "\n") == 1
    return text.replace(line + "\n", "", 1)

def add(text, line):
    return text.rstrip("\n") + "\n" + line + "\n"

user = line_starting("user-font\t", lambda l: len(l.split("\t")) == 4)
asset = line_starting("asset-font\t")
arial = line_starting("user-font\t", lambda l: l.split("\t")[3] == "/Library/Fonts/Arial Unicode.ttf")
modelled = line_starting("font\t", lambda l: l.endswith("/Helvetica.ttc"))
cascade = line_starting("font\t", lambda l: l.endswith("/Apple Color Emoji.ttc"))
cascade6 = line_starting("font\t", lambda l: l.endswith("/Supplemental/Songti.ttc"))
hira = line_starting("font\t", lambda l: "W6.ttc" in l)
def with_hash(row, h, idx):
    f = row.split("\t"); f[idx] = h; return "\t".join(f)

print("exit\texpected\tstatus\tcase")
# --- earlier rounds (review 1 and review 2 listed facts) ---
run("relay-copy", T, T, 0)
run("comment-only", "# note\n" + T, T, 0)
run("reordered-rows", "\n".join([l for l in lines if not l.startswith(("font\t","user-font\t","asset-font\t"))] + list(reversed([l for l in lines if l.startswith(("font\t","user-font\t","asset-font\t"))]))) + "\n", T, 0)
run("reordered-scalars", "\n".join([l for l in lines if l.startswith("#")] + list(reversed([l for l in lines if "=" in l and not l.startswith(("#","font\t","user-font\t","asset-font\t"))])) + [l for l in lines if l.startswith(("font\t","user-font\t","asset-font\t"))]) + "\n", T, 0)
run("extra-live-scalar", add(T, "extra.note=1"), T, 0)
run("catalog-xml-plist-live", add(add(T, "asset-font\t1\tabc\t/System/Library/AssetsV2/com_apple_MobileAsset_Font6/com_apple_MobileAsset_Font6.xml"), "asset-font\t1\tabc\t/System/Library/AssetsV2/com_apple_MobileAsset_Font8/x.asset/Info.plist"), T, 0)
run("harfbuzz-keg", once(T, "kegs=10.1.0,13.2.1,15.0.0_1", "kegs=10.1.0,13.2.1,15.0.0_1,16.0.0"), T, 1)
run("harfbuzz-dylib-hash", once(T, "b04ab259a343ba51a50db8d1df637160f31d342a66c4192e18dba63ea42ed04a", "0"*64), T, 1)
run("harfbuzz-opt-target", once(T, "opt_target=../Cellar/harfbuzz/15.0.0_1", "opt_target=../Cellar/harfbuzz/16.0.0"), T, 1)
run("harfbuzz-loaded", once(T, "harfbuzz.loaded=/opt/homebrew/Cellar/harfbuzz/15.0.0_1", "harfbuzz.loaded=/opt/homebrew/Cellar/harfbuzz/13.2.1"), T, 1)
run("harfbuzz-current-version", set_scalar(T, "shaping.harfbuzz.current_version", "61600.0.0"), T, 1)
run("harfbuzz-link-switched", set_scalar(T, "shaping.harfbuzz.link_switched", "2026-11-08"), T, 1)
run("freetype-hash", once(T, "1bf70387c4ef59accba8116c847cb87d2018ded2672db551686b95dce4c5d931", "1"*64), T, 1)
run("freetype-opt-target", set_scalar(T, "shaping.freetype.opt_target", "../Cellar/freetype/2.16.0"), T, 1)
run("freetype-link-current", set_scalar(T, "shaping.freetype.link_current", "27.7.0"), T, 1)
run("live-missing-freetype-key", set_scalar(T, "shaping.freetype.sha256", None), T, 1)
run("jdk-hash", once(T, "ddbdbe280c4d939de81f27f9c135613d0a250738ab2d15e8dec52fd7345354d9", "2"*64), T, 1)
run("jdk-home", set_scalar(T, "jdk.home", "/opt/homebrew/Cellar/openjdk@21/21.0.13.0.2/libexec/openjdk.jdk/Contents/Home"), T, 1)
run("fontmanager-hash", once(T, "80488062f9ab885f1860b0e3b8fc3d13030a0a05765c06d12301668d7404812c", "3"*64), T, 1)
run("reference-wrapper-hash", once(T, "6115a3503877999796cb98b9efb6872d9bb8090c2f7b986706919e475dee069e", "4"*64), T, 1)
run("os-build", set_scalar(T, "host.build_version", "26B124"), T, 1)
run("os-product", set_scalar(T, "host.product_version", "27.3"), T, 1)
run("hostname", set_scalar(T, "host.hostname", "colossus.lan"), T, 1)
run("user-font-hash", once(T, user, with_hash(user, "0"*64, 2)), T, 1)
run("user-font-size", once(T, user, with_hash(user, "1", 1)), T, 1)
run("user-font-removed", drop(T, user), T, 1)
run("user-font-added", add(T, "user-font\t3\t" + "cd"*32 + "\t/Library/Fonts/new.ttf"), T, 1)
run("asset-font-hash", once(T, asset, with_hash(asset, "ab"*32, 2)), T, 1)
run("asset-font-removed", drop(T, asset), T, 1)
run("asset-font-added", add(T, "asset-font\t4\t" + "ef"*32 + "\t/System/Library/AssetsV2/com_apple_MobileAsset_Font8/n.asset/AssetData/N.ttf"), T, 1)
run("modelled-font-hash", once(T, modelled, with_hash(modelled, "5"*64, 1)), T, 1)
run("modelled-font-removed", drop(T, modelled), T, 1)
run("modelled-font-added", add(T, "font\t" + "6"*64 + "\t/System/Library/Fonts/Supplemental/Georgia.ttf"), T, 1)
run("symlink-size", once(T, arial, with_hash(arial, "23278009", 1)), T, 1)
run("symlink-target-hash", once(T, arial, with_hash(arial, "7"*64, 2)), T, 1)
run("symlink-target-path", once(T, arial, with_hash(arial, "/Library/Fonts/Other.ttf", 4)), T, 1)
run("symlink-became-regular-file", once(T, arial, "\t".join(arial.split("\t")[:4])), T, 1)
run("symlink-dangling", once(T, arial, "\t".join(["user-font", "0", "dangling", "/Library/Fonts/Arial Unicode.ttf", ""])), T, 1)
uuid_row = line_starting("user-font\t", lambda l: "c02979894bd1113a75b668101ed8751d9885066289e70a12aedbd2e1c1576adb" in l)
run("uuid-user-font-hash", once(T, uuid_row, with_hash(uuid_row, "33"*32, 2)), T, 1)
# --- new: cascade faces ---
run("cascade-face-hash-round9", once(T, cascade, with_hash(cascade, "cd"*32, 1)), T, 1)
run("cascade-face-hash-round6-only", once(T, cascade6, with_hash(cascade6, "cd"*32, 1)), T, 1)
run("cascade-face-hash-nfd-name", once(T, hira, with_hash(hira, "cd"*32, 1)), T, 1)
run("cascade-face-removed", drop(T, cascade), T, 1)
run("cascade-face-added", add(T, "font\t" + "8"*64 + "\t/System/Library/Fonts/NewCascade.ttc"), T, 1)
run("cascade-face-nfc-renamed", once(T, hira, hira.replace("ギ", "ギ").replace("ゴ", "ゴ")), T, 1)
run("committed-census3-live-census4", T, Path(sys.argv[4]).read_text(encoding="utf-8"), 1)
# --- new: scalar blank / dash grid (key host.product_version) ---
key = "host.product_version"
values = {"value": "27.2", "other": "27.3", "blank": "", "dash": "-", "missing": None}
for (cn, cv), (ln, lv) in itertools.product(values.items(), values.items()):
    if cn in ("other",):
        continue
    if cv is None and lv is None:
        continue
    committed = set_scalar(T, key, cv)
    live = set_scalar(T, key, lv)
    if cv in ("", "-"):
        expect = 2
    elif cv is None:
        expect = 0  # committed has no such key: not pinned, extra live scalar is ignored
    else:
        expect = 0 if lv == cv else 1
    run(f"scalar-committed-{cn}-live-{ln}", live, committed, expect)
# freetype scalar special-case path
run("freetype-scalar-committed-blank-live-real", T, set_scalar(T, "shaping.freetype.sha256", ""), 2)
run("freetype-scalar-committed-missing-live-real", T, set_scalar(T, "shaping.freetype.sha256", None), 2)
run("freetype-scalar-committed-dash-live-dash", set_scalar(T, "shaping.freetype.sha256", "-"), set_scalar(T, "shaping.freetype.sha256", "-"), 2)
# --- new: row hash blank / dash grid ---
for kind, row, idx in (("user-font", user, 2), ("asset-font", asset, 2), ("symlink-row", arial, 2), ("font", cascade, 1)):
    real = row.split("\t")[idx]
    for (cn, cv), (ln, lv) in itertools.product((("real", real), ("blank", ""), ("dash", "-")), (("real", real), ("other", "9"*64), ("blank", ""), ("dash", "-"))):
        committed = once(T, row, with_hash(row, cv, idx)) if cv != real else T
        live = once(T, row, with_hash(row, lv, idx)) if lv != real else T
        if cv in ("", "-"):
            expect = 2
        else:
            expect = 0 if lv == cv else 1
        run(f"{kind}-hash-committed-{cn}-live-{ln}", live, committed, expect)
# mixed: unpinned plus real drift reports drift (1)
run("dash-scalar-plus-real-drift", once(set_scalar(T, key, "-"), "b04ab259a343ba51a50db8d1df637160f31d342a66c4192e18dba63ea42ed04a", "0"*64), set_scalar(T, key, "-"), 1)
