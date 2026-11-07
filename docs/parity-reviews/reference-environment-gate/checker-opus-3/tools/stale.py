"""Mutate CASCADE_FONT_FILES and the stored #slot data in a mock checkout; record self-test exit codes."""
import shutil, subprocess, sys
from pathlib import Path
repo, base = Path(sys.argv[1]), Path(sys.argv[2])
base.mkdir(parents=True)
def mock(name):
    root = base / name
    (root / "crates").mkdir(parents=True)
    (root / "scripts").mkdir()
    shutil.copy(repo / "Cargo.toml", root / "Cargo.toml")
    shutil.copy(repo / "scripts/reference_env_census.py", root / "scripts/reference_env_census.py")
    shutil.copytree(repo / "tools/font-chain-reference", root / "tools/font-chain-reference")
    return root
def run(name, expect_ok, mutate):
    root = mock(name)
    mutate(root)
    p = subprocess.run([sys.executable, "-I", str(root / "scripts/reference_env_census.py"), "self-test"], capture_output=True, text=True)
    last = (p.stdout.strip().splitlines() or [""])[-1] if p.returncode == 0 else (p.stderr.strip().splitlines() or [""])[-1]
    ok = (p.returncode == 0) == expect_ok
    print(f"{p.returncode}\t{'0' if expect_ok else 'nonzero'}\t{'ok' if ok else 'UNEXPECTED'}\t{name}\t{last[:160]}")
    shutil.rmtree(root)
def edit(path, old, new, count=1):
    s = path.read_text(encoding="utf-8")
    assert s.count(old) >= 1, (path, old)
    path.write_text(s.replace(old, new, count) if count else s.replace(old, new), encoding="utf-8")
S = lambda r: r / "scripts/reference_env_census.py"
CH = lambda r: r / "tools/font-chain-reference"
def every_tsv(r, fn):
    for t in CH(r).rglob("*.tsv"):
        t.write_text(fn(t.read_text(encoding="utf-8")), encoding="utf-8")
print("exit\texpected\tstatus\tcase\tlast line")
run("unchanged", True, lambda r: None)
run("tuple-drop-songti", False, lambda r: edit(S(r), '    "/System/Library/Fonts/Supplemental/Songti.ttc",\n', ""))
run("tuple-add-georgia", False, lambda r: edit(S(r), '    "/System/Library/Fonts/Supplemental/Songti.ttc",\n', '    "/System/Library/Fonts/Supplemental/Songti.ttc",\n    "/System/Library/Fonts/Supplemental/Georgia.ttf",\n'))
run("tuple-swap-order", False, lambda r: edit(S(r), '    "/System/Library/Fonts/Apple Color Emoji.ttc",\n    "/System/Library/Fonts/Apple Symbols.ttf",\n', '    "/System/Library/Fonts/Apple Symbols.ttf",\n    "/System/Library/Fonts/Apple Color Emoji.ttc",\n'))
run("tuple-nfc-hiragino", False, lambda r: edit(S(r), "\\u30ad\\u3099\\u30ce\\u660e\\u671d", "\\u30ae\\u30ce\\u660e\\u671d"))
run("slot-add-new-path", False, lambda r: edit(CH(r) / "monkeyman-round9/Helvetica.tsv", "#slot\tAppleColorEmoji\t", "#slot\tGeorgia\t/System/Library/Fonts/Supplemental/Georgia.ttf\tsystem\tregistered\n#slot\tAppleColorEmoji\t"))
run("slot-remove-round6-only-songti", False, lambda r: every_tsv(r, lambda s: "\n".join(l for l in s.split("\n") if not (l.startswith("#slot\t") and "/Songti.ttc" in l))))
run("slot-path-to-user-dir", False, lambda r: every_tsv(r, lambda s: s.replace("/System/Library/Fonts/Supplemental/Kokonor.ttf", "/Library/Fonts/Kokonor.ttf")))
run("slot-path-to-modelled-face", False, lambda r: every_tsv(r, lambda s: s.replace("/System/Library/Fonts/Supplemental/Kokonor.ttf", "/System/Library/Fonts/Helvetica.ttc")))
run("slot-path-relative", False, lambda r: edit(CH(r) / "monkeyman-round9/Helvetica.tsv", "\t/System/Library/Fonts/Apple Color Emoji.ttc\t", "\tApple Color Emoji.ttc\t"))
run("slot-lines-all-removed", False, lambda r: every_tsv(r, lambda s: "\n".join(l for l in s.split("\n") if not l.startswith("#slot\t"))))
run("round9-emoji-removed", False, lambda r: every_tsv(r, lambda s: "\n".join(l for l in s.split("\n") if not (l.startswith("#slot\t") and "Apple Color Emoji" in l))))
run("slot-lines-reordered", True, lambda r: every_tsv(r, lambda s: "\n".join([l for l in s.split("\n") if not l.startswith("#slot\t")] + list(reversed([l for l in s.split("\n") if l.startswith("#slot\t")])))))
run("tsv-renamed", True, lambda r: (CH(r) / "monkeyman-round9/Helvetica.tsv").rename(CH(r) / "monkeyman-round9/zz-Helvetica.tsv"))
run("non-tsv-slot-file-added", True, lambda r: (CH(r) / "monkeyman-round9/notes.txt").write_text("#slot\tX\t/System/Library/Fonts/X.ttc\tsystem\tregistered\n", encoding="utf-8"))
run("census-drop-emoji-row", False, lambda r: edit(CH(r) / "monkeyman-reference-env/census.txt", "font\t12bc8221a908c5bcf05f0fce940d3c3f6e9d09d8d806ffe31a969b4ac9be8f9e\t/System/Library/Fonts/Apple Color Emoji.ttc\n", ""))
run("census-drop-songti-row (stale census; gate fails closed at check)", True, lambda r: edit(CH(r) / "monkeyman-reference-env/census.txt", next(l for l in (CH(r) / "monkeyman-reference-env/census.txt").read_text(encoding="utf-8").splitlines() if l.endswith("/Songti.ttc")) + "\n", ""))
