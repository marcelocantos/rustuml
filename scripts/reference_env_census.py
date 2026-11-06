#!/usr/bin/env python3
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Read-only census of the Monkey Man Java reference environment.

The owner decision of 2026-11-06 pins that host (fonts, Homebrew HarfBuzz
15.0.0_1, FreeType 2.15.0, OpenJDK 21.0.5) as the oracle. This script
records a live census and compares it to the committed one. It never
installs, upgrades or rewrites anything outside the output directory
passed to ``record``.

Modes
-----
record OUTDIR
    Run on Monkey Man. Writes only under OUTDIR.
check LIVE_DIR --committed CENSUS
    Compare a census written by ``record`` to the committed census.
    Exit 0: every pinned fact matches.
    Exit 2: pinned facts match, but a hash the baseline has not pinned yet
    is present only on the live side (or still ``-``).
    Exit 1: a pinned fact differs, or a font/user-font/asset path or size moved.
assemble OUTFILE
    Rebuild the committed census from the stored reference artifacts in
    this checkout. Used to regenerate ``census.txt``; not run on Monkey Man.
self-test
    Check the committed census against a copy of itself, then against a
    one-byte drift.

``python3 -I`` is the intended invocation.
"""

from __future__ import annotations

import datetime
import hashlib
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

SCHEMA = "1"
PROBE = b"@startuml\nAlice -> Bob: census\n@enduml\n"
REFERENCE = "/Users/marcelo/rustuml-reference/bin/reference"
JDK_HOME = "/opt/homebrew/Cellar/openjdk@21/21.0.5/libexec/openjdk.jdk/Contents/Home"

# Faces the renderer models, plus the extra font files the stored Monkey Man
# extractions already hash (Times, Tahoma, Geeza Pro). ``record`` hashes these
# paths; ``assemble`` copies their hashes out of the stored records.
FONT_FILES = (
    "/System/Library/Fonts/GeezaPro.ttc",
    "/System/Library/Fonts/Helvetica.ttc",
    "/System/Library/Fonts/LucidaGrande.ttc",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf",
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/System/Library/Fonts/Supplemental/Arial Italic.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/System/Library/Fonts/Supplemental/Courier New Bold Italic.ttf",
    "/System/Library/Fonts/Supplemental/Courier New Bold.ttf",
    "/System/Library/Fonts/Supplemental/Courier New Italic.ttf",
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
    "/System/Library/Fonts/Supplemental/Tahoma Bold.ttf",
    "/System/Library/Fonts/Supplemental/Tahoma.ttf",
    "/System/Library/Fonts/Supplemental/Times New Roman Bold Italic.ttf",
    "/System/Library/Fonts/Supplemental/Times New Roman Bold.ttf",
    "/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf",
    "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
    "/System/Library/Fonts/Supplemental/Verdana Bold Italic.ttf",
    "/System/Library/Fonts/Supplemental/Verdana Bold.ttf",
    "/System/Library/Fonts/Supplemental/Verdana Italic.ttf",
    "/System/Library/Fonts/Supplemental/Verdana.ttf",
)

# An asset font is a font file. Catalog XML and per-asset Info.plist
# files sit in the same MobileAsset trees and are not faces.
ASSET_FONT_SUFFIXES = {".ttf", ".otf", ".ttc", ".dfont"}

USER_FONT_DIRS = (
    Path.home() / "Library" / "Fonts",
    Path("/Library/Fonts"),
)
ASSET_FONT_DIRS = (
    Path("/System/Library/AssetsV2/com_apple_MobileAsset_Font6"),
    Path("/System/Library/AssetsV2/com_apple_MobileAsset_Font7"),
    Path("/System/Library/AssetsV2/com_apple_MobileAsset_Font8"),
)

LS_LINE = re.compile(
    r"^[-l][rwxsStT-]{9}[@+ ]?\s+\d+\s+\S+\s+\S+\s+(\d+)\s+"
    r"\S+\s+\d+\s+(?:\d{4}|\d{2}:\d{2})\s+(.+)$"
)
LOADED_LINE = re.compile(r"^\S+:\s+<[0-9A-Fa-f-]+>\s+(\S.*\.(?:dylib|so))$")
SHA_LINE = re.compile(r"^([0-9a-f]{64})  (\S.*)$")

# Scalar keys, in the order ``record`` and ``assemble`` write them.
# A value of "" means the artifact capture did not include it.
SCALAR_ORDER = (
    "host.hostname",
    "host.product_name",
    "host.product_version",
    "host.build_version",
    "jdk.home",
    "jdk.java",
    "jdk.java.sha256",
    "jdk.fontmanager",
    "jdk.fontmanager.sha256",
    "jdk.reference_wrapper",
    "jdk.reference_wrapper.sha256",
    "shaping.harfbuzz.opt_link",
    "shaping.harfbuzz.opt_target",
    "shaping.harfbuzz.link_switched",
    "shaping.harfbuzz.kegs",
    "shaping.harfbuzz.loaded",
    "shaping.harfbuzz.sha256",
    "shaping.harfbuzz.current_version",
    "shaping.freetype.loaded",
    "shaping.freetype.sha256",
    "shaping.freetype.link_compat",
    "shaping.freetype.link_current",
    "shaping.fontmanager.loaded",
)


def repo_root() -> Path | None:
    here = Path(__file__).resolve()
    for candidate in (here.parent, *here.parents):
        if (candidate / "Cargo.toml").is_file() and (candidate / "crates").is_dir():
            return candidate
    return None


def default_committed() -> Path:
    root = repo_root()
    if root is None:
        return Path(__file__).resolve().with_name("census.txt")
    return root / "tools/font-chain-reference/monkeyman-reference-env/census.txt"


def is_asset_font(path: Path) -> bool:
    """True for a font file. Shared by ``record``, ``assemble`` and ``check``."""
    return path.suffix.lower() in ASSET_FONT_SUFFIXES


def asset_font_rows(rows: list[tuple[str, ...]]) -> list[tuple[str, ...]]:
    """Drop MobileAsset catalog files. Font and user-font rows stay."""
    kept = []
    for row in rows:
        if row[0] == "asset-font" and not is_asset_font(Path(row[-1])):
            continue
        kept.append(row)
    return kept


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_census(text: str) -> tuple[dict[str, str], list[tuple[str, ...]]]:
    scalars: dict[str, str] = {}
    rows: list[tuple[str, ...]] = []
    for line_no, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith(("font\t", "user-font\t", "asset-font\t")):
            fields = tuple(line.split("\t"))
            if fields[0] == "font" and len(fields) != 3:
                raise SystemExit(f"census:{line_no}: font row needs sha256 and path")
            if fields[0] != "font" and len(fields) != 4:
                raise SystemExit(f"census:{line_no}: {fields[0]} row needs size, sha256 and path")
            rows.append(fields)
            continue
        if "=" not in line:
            raise SystemExit(f"census:{line_no}: expected key=value or a row")
        key, value = line.split("=", 1)
        if key in scalars:
            raise SystemExit(f"census:{line_no}: duplicate key {key}")
        scalars[key] = value
    return scalars, rows


def format_census(scalars: dict[str, str], rows: list[tuple[str, ...]], header: list[str]) -> str:
    lines = [f"# rustuml reference-environment census schema {SCHEMA}"]
    lines.extend(header)
    for key in SCALAR_ORDER:
        if key in scalars:
            lines.append(f"{key}={scalars[key]}")
    extra = sorted(set(scalars) - set(SCALAR_ORDER))
    for key in extra:
        lines.append(f"{key}={scalars[key]}")
    for row in sorted(rows):
        lines.append("\t".join(row))
    lines.append("")
    return "\n".join(lines)


def font_hashes_in(path: Path) -> dict[str, str]:
    found: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = SHA_LINE.match(line)
        if match is None:
            continue
        digest, name = match.group(1), match.group(2)
        if name.startswith("/System/Library/Fonts/"):
            previous = found.get(name)
            if previous is not None and previous != digest:
                raise SystemExit(f"{path}: {name} hashed twice as {previous} and {digest}")
            found[name] = digest
    return found


def parse_ls_census(path: Path) -> list[tuple[str, str, str]]:
    """Return (size, sha_or_marker, full_path) from an ``ls -la`` census."""
    current = ""
    rows: list[tuple[str, str, str]] = []
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if raw.startswith("## "):
            current = raw[3:].strip()
            continue
        match = LS_LINE.match(raw)
        if match is None:
            continue
        size, name = match.group(1), match.group(2)
        marker = "-"
        if raw.startswith("l"):
            marker = "link"
            name = name.split(" -> ", 1)[0]
        full = name if name.startswith("/") else str(Path(current) / name)
        rows.append((size, marker, full))
    return rows


def assemble(outfile: Path) -> None:
    root = repo_root()
    if root is None:
        raise SystemExit("assemble must run inside the rustuml checkout")
    chain = root / "tools/font-chain-reference"
    sources = [
        chain / "monkeyman-round9/font-hashes.txt",
        chain / "ascii-advances-round11/font-hashes.txt",
        chain / "colossus-primary-faces/monkeyman-round7-font-hash.txt",
        chain / "colossus-primary-faces/monkeyman-round8-font-hash.txt",
        chain / "colossus-primary-faces/monkeyman-hb-static.txt",
    ]
    hashes: dict[str, str] = {}
    for source in sources:
        for path, digest in font_hashes_in(source).items():
            previous = hashes.get(path)
            if previous is not None and previous != digest:
                raise SystemExit(f"{path}: {source.name} disagrees with an earlier record")
            hashes[path] = digest
    missing = [path for path in FONT_FILES if path not in hashes]
    if missing:
        raise SystemExit("stored artifacts do not hash: " + ", ".join(missing))
    static = (chain / "colossus-primary-faces/monkeyman-hb-static.txt").read_text(encoding="utf-8")
    loaded = (chain / "monkeyman-reference-env/methodA.loaded.txt").read_text(encoding="utf-8")
    loaded_paths = []
    for line in loaded.splitlines():
        match = LOADED_LINE.match(line.strip())
        if match:
            loaded_paths.append(match.group(1))

    def loaded_ending(suffix: str) -> str:
        hits = [path for path in loaded_paths if path.endswith(suffix)]
        if len(hits) != 1:
            raise SystemExit(f"methodA.loaded.txt has {hits} for *{suffix}")
        return hits[0]

    def static_sha(suffix: str) -> str:
        for line in static.splitlines():
            match = SHA_LINE.match(line)
            if match and match.group(2).endswith(suffix):
                return match.group(1)
        raise SystemExit(f"static capture has no sha256 ending {suffix}")

    sw = (chain / "monkeyman-round6/sw_vers.txt").read_text(encoding="utf-8")
    product = {}
    for line in sw.splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            product[key.strip()] = value.strip()
    hostname = (chain / "monkeyman-round6/hostname.txt").read_text(encoding="utf-8").strip()
    scalars = {
        "host.hostname": hostname,
        "host.product_name": product["ProductName"],
        "host.product_version": product["ProductVersion"],
        "host.build_version": product["BuildVersion"],
        "jdk.home": JDK_HOME,
        "jdk.java": JDK_HOME + "/bin/java",
        "jdk.java.sha256": static_sha("/bin/java"),
        "jdk.fontmanager": JDK_HOME + "/lib/libfontmanager.dylib",
        "jdk.fontmanager.sha256": static_sha("/lib/libfontmanager.dylib"),
        "jdk.reference_wrapper": REFERENCE,
        "jdk.reference_wrapper.sha256": static_sha("/bin/reference"),
        "shaping.harfbuzz.opt_link": "/opt/homebrew/opt/harfbuzz",
        "shaping.harfbuzz.opt_target": "../Cellar/harfbuzz/15.0.0_1",
        "shaping.harfbuzz.link_switched": "2026-10-28",
        "shaping.harfbuzz.kegs": "10.1.0,13.2.1,15.0.0_1",
        "shaping.harfbuzz.loaded": loaded_ending("/libharfbuzz.0.dylib"),
        "shaping.harfbuzz.sha256": static_sha("/libharfbuzz.0.dylib"),
        "shaping.harfbuzz.current_version": "61500.0.0",
        "shaping.freetype.loaded": loaded_ending("/libfreetype.6.dylib"),
        # The round-6 capture names the loaded FreeType path and the
        # link-time version. It does not hash the dylib. Left empty until
        # a live census is committed.
        "shaping.freetype.sha256": "",
        "shaping.freetype.link_compat": "27.0.0",
        "shaping.freetype.link_current": "27.2.0",
        "shaping.fontmanager.loaded": loaded_ending("/libfontmanager.dylib"),
    }
    rows: list[tuple[str, ...]] = [("font", hashes[path], path) for path in FONT_FILES]
    for size, marker, path in parse_ls_census(chain / "monkeyman-round6/user-font-census.txt"):
        rows.append(("user-font", size, marker, path))
    for size, marker, path in parse_ls_census(chain / "monkeyman-round6/asset-font-census.txt"):
        rows.append(("asset-font", size, marker, path))
    rows = asset_font_rows(rows)
    header = [
        "# origin: stored Monkey Man relay artifacts, not a live re-hash",
        "# harfbuzz load: shared-svg-round6-monkeyman-hb SHA256SUMS c5d00e3cd71cd98cfb546c940ac747ef1a9281fc33cf0a10d1c010a63314adc8",
        "# font chain: shared-svg-round6-monkeyman-fontchain SHA256SUMS 1c345cc384f0edb939dac3b87cf135db67ed7b6f8ffb76b586a8f9fd8480a6ed",
        "# styles: shared-svg-round9-mm-styles SHA256SUMS ad8e6566043819726c0146b76261b2aea3710eb83372d59547b46c3132f3ed55",
        "# ascii: shared-svg-round11-mm-ascii SHA256SUMS f5def121617419f6d55ed7361fdba5b1d0e0b90433766f1e662a0c81586573d1",
        "# round8 font hash file sha256 a2ef54cecc47e0e8426043224a6cd9964c453e9f459002147acb45c825c626dd",
        "# FreeType dylib sha256 is unpinned (empty) until a live record lands.",
        "# Do not brew upgrade on Monkey Man. A keg change moves this oracle.",
    ]
    outfile.write_text(format_census(scalars, rows, header), encoding="utf-8")


def otool_current_version(path: str) -> str:
    text = run_checked(["otool", "-L", path])
    match = re.search(r"current version ([0-9.]+)", text)
    if match is None:
        raise SystemExit(f"otool -L {path} did not report a current version")
    return match.group(1)


def otool_dependency_versions(path: str, token: str) -> tuple[str, str]:
    """Compatibility and current versions of the ``token`` dependency."""
    text = run_checked(["otool", "-L", path])
    for line in text.splitlines():
        if token not in line:
            continue
        compat = re.search(r"compatibility version ([0-9.]+)", line)
        current = re.search(r"current version ([0-9.]+)", line)
        if compat and current:
            return compat.group(1), current.group(1)
    raise SystemExit(f"otool -L {path} has no {token} dependency")


def run_checked(command: list[str], env: dict[str, str] | None = None) -> str:
    completed = subprocess.run(command, check=False, capture_output=True, env=env, text=True)
    if completed.returncode != 0:
        raise SystemExit(
            f"{command[0]} exited {completed.returncode}: {completed.stderr.strip()}"
        )
    return completed.stdout


def record(outdir: Path) -> None:
    if outdir.exists() and any(outdir.iterdir()):
        raise SystemExit(f"{outdir} is not empty; record writes only into an empty directory")
    outdir.mkdir(parents=True, exist_ok=True)
    # Every path below is under outdir. Reads elsewhere are explicit.
    sw = run_checked(["sw_vers"])
    (outdir / "sw_vers.txt").write_text(sw, encoding="utf-8")
    hostname = run_checked(["hostname"]).strip()
    product = {}
    for line in sw.splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            product[key.strip()] = value.strip()
    java = JDK_HOME + "/bin/java"
    fontmanager = JDK_HOME + "/lib/libfontmanager.dylib"
    opt_hb = Path("/opt/homebrew/opt/harfbuzz")
    opt_ft = Path("/opt/homebrew/opt/freetype")
    hb_kegs = sorted(path.name for path in Path("/opt/homebrew/Cellar/harfbuzz").iterdir() if path.is_dir())
    static_lines = [
        f"hostname {hostname}",
        sw.rstrip(),
        f"harfbuzz-link {os.path.realpath(opt_hb) if opt_hb.exists() else 'missing'}",
        f"harfbuzz-readlink {os.readlink(opt_hb) if opt_hb.is_symlink() else 'missing'}",
        f"freetype-link {os.path.realpath(opt_ft) if opt_ft.exists() else 'missing'}",
        f"freetype-readlink {os.readlink(opt_ft) if opt_ft.is_symlink() else 'missing'}",
        "harfbuzz-kegs " + ",".join(hb_kegs),
    ]
    (outdir / "static.txt").write_text("\n".join(static_lines) + "\n", encoding="utf-8")

    env = os.environ.copy()
    env["DYLD_PRINT_LIBRARIES"] = "1"
    completed = subprocess.run(
        [REFERENCE, "--cwd", str(outdir), "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"],
        input=PROBE,
        capture_output=True,
        env=env,
        check=False,
    )
    (outdir / "methodA.svg").write_bytes(completed.stdout)
    (outdir / "methodA.stderr").write_bytes(completed.stderr)
    (outdir / "methodA.exit").write_text(f"exit {completed.returncode}\n", encoding="utf-8")
    if completed.returncode != 0:
        raise SystemExit(f"reference JVM exited {completed.returncode}; see {outdir}/methodA.stderr")
    loaded_paths = []
    loaded_text = []
    for raw in completed.stderr.decode("utf-8", errors="replace").splitlines():
        match = LOADED_LINE.match(raw.strip())
        if match is None:
            continue
        path = match.group(1)
        if any(token in path.lower() for token in ("harfbuzz", "freetype", "fontmanager")):
            loaded_text.append(raw.strip())
            loaded_paths.append(path)
    (outdir / "methodA.loaded.txt").write_text("\n".join(loaded_text) + "\n", encoding="utf-8")

    def one_loaded(token: str) -> str:
        hits = [path for path in loaded_paths if token in path]
        if len(hits) != 1:
            raise SystemExit(f"DYLD_PRINT_LIBRARIES showed {hits} for {token}")
        return hits[0]

    hb_loaded = one_loaded("libharfbuzz.0.dylib")
    ft_loaded = one_loaded("libfreetype.6.dylib")
    fm_loaded = one_loaded("libfontmanager.dylib")
    link_switched = ""
    if opt_hb.is_symlink():
        link_switched = datetime.datetime.fromtimestamp(opt_hb.lstat().st_mtime).strftime("%Y-%m-%d")
    ft_compat, ft_current = otool_dependency_versions(fontmanager, "libfreetype")
    scalars = {
        "host.hostname": hostname,
        "host.product_name": product.get("ProductName", ""),
        "host.product_version": product.get("ProductVersion", ""),
        "host.build_version": product.get("BuildVersion", ""),
        "jdk.home": JDK_HOME,
        "jdk.java": java,
        "jdk.java.sha256": sha256_file(Path(java)),
        "jdk.fontmanager": fontmanager,
        "jdk.fontmanager.sha256": sha256_file(Path(fontmanager)),
        "jdk.reference_wrapper": REFERENCE,
        "jdk.reference_wrapper.sha256": sha256_file(Path(REFERENCE)),
        "shaping.harfbuzz.opt_link": str(opt_hb),
        "shaping.harfbuzz.opt_target": os.readlink(opt_hb) if opt_hb.is_symlink() else "",
        "shaping.harfbuzz.link_switched": link_switched,
        "shaping.harfbuzz.kegs": ",".join(hb_kegs),
        "shaping.harfbuzz.loaded": hb_loaded,
        "shaping.harfbuzz.sha256": sha256_file(Path(hb_loaded)),
        "shaping.harfbuzz.current_version": otool_current_version(hb_loaded),
        "shaping.freetype.loaded": ft_loaded,
        "shaping.freetype.sha256": sha256_file(Path(ft_loaded)),
        "shaping.freetype.link_compat": ft_compat,
        "shaping.freetype.link_current": ft_current,
        "shaping.fontmanager.loaded": fm_loaded,
    }
    if opt_ft.is_symlink():
        scalars["shaping.freetype.opt_target"] = os.readlink(opt_ft)
    rows: list[tuple[str, ...]] = []
    for path in FONT_FILES:
        rows.append(("font", sha256_file(Path(path)), path))
    for directory in USER_FONT_DIRS:
        if not directory.is_dir():
            continue
        for name in sorted(os.listdir(directory)):
            full = directory / name
            if not full.is_symlink() and not full.is_file():
                continue
            listed = full.lstat().st_size
            if full.is_symlink():
                rows.append(("user-font", str(listed), "link", str(full)))
            else:
                rows.append(("user-font", str(listed), sha256_file(full), str(full)))
    for directory in ASSET_FONT_DIRS:
        if not directory.is_dir():
            continue
        for dirpath, dirnames, filenames in os.walk(directory):
            dirnames.sort()
            for name in sorted(filenames):
                full = Path(dirpath) / name
                if not full.is_file() or full.is_symlink():
                    continue
                # Hash only after the shared font-file test, so a catalog
                # file is neither hashed nor written.
                if not is_asset_font(full):
                    continue
                rows.append(("asset-font", str(full.stat().st_size), sha256_file(full), str(full)))
    rows = asset_font_rows(rows)
    header = [
        "# origin: live Monkey Man record",
        "# written only under the output directory of this invocation",
    ]
    census = format_census(scalars, rows, header)
    (outdir / "census.txt").write_text(census, encoding="utf-8")
    digest = hashlib.sha256(census.encode("utf-8")).hexdigest()
    (outdir / "SHA256SUMS").write_text(f"{digest}  census.txt\n", encoding="utf-8")
    print(f"recorded {outdir / 'census.txt'}")


def index_rows(rows: list[tuple[str, ...]]) -> dict[str, dict[str, tuple[str, ...]]]:
    grouped: dict[str, dict[str, tuple[str, ...]]] = {"font": {}, "user-font": {}, "asset-font": {}}
    for row in rows:
        kind = row[0]
        path = row[-1]
        if path in grouped[kind]:
            raise SystemExit(f"duplicate {kind} path {path}")
        grouped[kind][path] = row
    return grouped


def check(live_dir: Path, committed_path: Path) -> int:
    live_scalars, live_rows = parse_census((live_dir / "census.txt").read_text(encoding="utf-8"))
    committed_scalars, committed_rows = parse_census(committed_path.read_text(encoding="utf-8"))
    live_rows = asset_font_rows(live_rows)
    committed_rows = asset_font_rows(committed_rows)
    live = index_rows(live_rows)
    committed = index_rows(committed_rows)
    drift: list[str] = []
    unpinned: list[str] = []
    for key, expected in sorted(committed_scalars.items()):
        if key not in live_scalars:
            drift.append(f"missing {key}")
            continue
        actual = live_scalars[key]
        if expected == "":
            if actual == "":
                unpinned.append(key)
            else:
                unpinned.append(f"{key} live={actual}")
            continue
        if actual != expected:
            drift.append(f"{key}: committed {expected} live {actual}")
    for kind in ("font", "user-font", "asset-font"):
        committed_paths = set(committed[kind])
        live_paths = set(live[kind])
        for path in sorted(committed_paths - live_paths):
            drift.append(f"missing {kind} {path}")
        for path in sorted(live_paths - committed_paths):
            drift.append(f"extra {kind} {path}")
        for path in sorted(committed_paths & live_paths):
            old, new = committed[kind][path], live[kind][path]
            if kind == "font":
                if old[1] != new[1]:
                    drift.append(f"font {path}: committed {old[1]} live {new[1]}")
                continue
            if old[1] != new[1]:
                drift.append(f"{kind} size {path}: committed {old[1]} live {new[1]}")
            old_hash, new_hash = old[2], new[2]
            if old_hash in ("", "-"):
                if new_hash in ("", "-"):
                    unpinned.append(f"{kind} {path}")
                else:
                    unpinned.append(f"{kind} {path} live={new_hash}")
            elif old_hash != new_hash:
                drift.append(f"{kind} sha256 {path}: committed {old_hash} live {new_hash}")
    if "shaping.freetype.sha256" not in committed_scalars or committed_scalars["shaping.freetype.sha256"] == "":
        live_hash = live_scalars.get("shaping.freetype.sha256", "")
        if live_hash and not any(item.startswith("shaping.freetype.sha256") for item in unpinned):
            unpinned.append(f"shaping.freetype.sha256 live={live_hash}")
    for item in drift:
        print(f"DRIFT {item}")
    for item in unpinned:
        print(f"UNPINNED {item}")
    if drift:
        print(f"{len(drift)} pinned facts differ")
        return 1
    if unpinned:
        print(f"pinned facts match; {len(unpinned)} hashes are not in the baseline yet")
        return 2
    print("pinned facts match")
    return 0


def self_test() -> None:
    committed = default_committed()
    if not committed.is_file():
        raise SystemExit(f"no committed census at {committed}")
    text = committed.read_text(encoding="utf-8")
    with tempfile.TemporaryDirectory() as tmp:
        live = Path(tmp)
        (live / "census.txt").write_text(text, encoding="utf-8")
        same = check(live, committed)
        if same != 0:
            raise SystemExit(f"self-check of the committed census exited {same}, expected 0")
        drifted = text.replace(
            "658718d77497145281cb0ec45ff2fe7c6e18062b2a38f837d0d792d9be3a9ba3",
            "0000000000000000000000000000000000000000000000000000000000000000",
            1,
        )
        (live / "census.txt").write_text(drifted, encoding="utf-8")
        changed = check(live, committed)
        if changed != 1:
            raise SystemExit(f"drift self-check exited {changed}, expected 1")
        # Catalog files share the MobileAsset tree. They are not faces, and
        # a record that lists them must not look like environment drift.
        catalog = text.rstrip("\n") + (
            "\nasset-font\t1138692\t-\t"
            "/System/Library/AssetsV2/com_apple_MobileAsset_Font6/"
            "com_apple_MobileAsset_Font6.xml\n"
            "asset-font\t1011\t-\t"
            "/System/Library/AssetsV2/com_apple_MobileAsset_Font8/"
            "00bfc46ccb002b730e29def5116e0a571fb617d8.asset/Info.plist\n"
        )
        (live / "census.txt").write_text(catalog, encoding="utf-8")
        catalog_check = check(live, committed)
        if catalog_check != same:
            raise SystemExit(
                f"catalog files changed the check from {same} to {catalog_check}"
            )
    print("self-test ok")


def main(argv: list[str]) -> int:
    if len(argv) < 2 or argv[1] in {"-h", "--help"}:
        print(__doc__)
        return 0
    mode = argv[1]
    if mode == "record":
        if len(argv) != 3:
            raise SystemExit("usage: reference_env_census.py record OUTDIR")
        record(Path(argv[2]))
        return 0
    if mode == "check":
        if len(argv) != 3 and not (len(argv) == 5 and argv[3] == "--committed"):
            raise SystemExit("usage: reference_env_census.py check LIVE_DIR --committed CENSUS")
        live = Path(argv[2])
        committed = Path(argv[4]) if len(argv) == 5 else default_committed()
        return check(live, committed)
    if mode == "assemble":
        if len(argv) != 3:
            raise SystemExit("usage: reference_env_census.py assemble OUTFILE")
        assemble(Path(argv[2]))
        return 0
    if mode == "self-test":
        self_test()
        return 0
    raise SystemExit(f"unknown mode {mode}")


if __name__ == "__main__":
    sys.exit(main(sys.argv))
