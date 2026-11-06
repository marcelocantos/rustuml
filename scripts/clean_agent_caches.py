#!/usr/bin/env python3
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Reclaim disk space left behind by parity maker/checker runs.

Parity rounds build several release trees each (base, merge, tip, drill
variants) and export whole source trees, golden corpus included, with
`git archive`. Left in place, these fill the disk: on 2026-11-06 the agent
caches reached 1.6 TB and runs failed with ENOSPC.

Only two kinds of directory are removed, because both regenerate exactly
from git:

  * Cargo build directories, identified by Cargo's own CACHEDIR.TAG.
  * Exported source trees: a directory holding Cargo.toml and crates/ but
    no .git entry (a `git archive` export, not a checkout).

Search roots: ~/Library/Caches/rustuml-* (never rustuml-relay-parent, which
holds the immutable Java references), target/ in each T3 worktree under
~/.t3/worktrees/rustuml, and /tmp/integ-*. Anything modified within
--min-age minutes is skipped so that a running delegate's trees survive.
Evidence, manifests, relay bundles, predictions and built binaries are
never touched.

Dry run by default; pass --apply to delete.
"""

import argparse
import shutil
import sys
import time
from pathlib import Path

HOME = Path.home()
CACHE_GLOB = (HOME / "Library" / "Caches", "rustuml-*")
PROTECTED_CACHES = {"rustuml-relay-parent"}
WORKTREE_ROOT = HOME / ".t3" / "worktrees" / "rustuml"
TMP_GLOB = (Path("/tmp"), "integ-*")
MAX_SEARCH_DEPTH = 3
BYTES_PER_GB = 1024**3


def is_cargo_target(d: Path) -> bool:
    """A Cargo build directory.

    Cargo writes CACHEDIR.TAG only when it creates the directory itself, so a
    CARGO_TARGET_DIR made beforehand (mkdir -p) has no tag. Those are
    recognised by Cargo's own .rustc_info.json plus a profile directory
    holding .fingerprint.
    """
    tag = d / "CACHEDIR.TAG"
    try:
        if tag.is_file() and "cargo" in tag.read_text(errors="replace").lower():
            return True
    except OSError:
        return False
    if not (d / ".rustc_info.json").is_file():
        return False
    try:
        return any((child / ".fingerprint").is_dir() for child in d.iterdir() if child.is_dir())
    except OSError:
        return False


def is_source_export(d: Path) -> bool:
    return (d / "Cargo.toml").is_file() and (d / "crates").is_dir() and not (d / ".git").exists()


def newest_mtime(d: Path) -> float:
    """Newest mtime of the directory and its immediate children.

    Shallow on purpose: walking a multi-GB tree to decide whether to
    delete it would cost as much as the deletion.
    """
    newest = d.stat().st_mtime
    try:
        for child in d.iterdir():
            try:
                newest = max(newest, child.lstat().st_mtime)
            except OSError:
                pass
    except OSError:
        pass
    return newest


def size_bytes(d: Path) -> int:
    total = 0
    for p in d.rglob("*"):
        try:
            if p.is_file() and not p.is_symlink():
                total += p.lstat().st_size
        except OSError:
            pass
    return total


def find_candidates(root: Path, depth: int):
    """Yield regenerable directories under root, without descending into them."""
    if depth > MAX_SEARCH_DEPTH or not root.is_dir() or root.is_symlink():
        return
    if is_cargo_target(root) or is_source_export(root):
        yield root
        return
    try:
        children = sorted(c for c in root.iterdir() if c.is_dir() and not c.is_symlink())
    except OSError:
        return
    for child in children:
        yield from find_candidates(child, depth + 1)


def search_roots():
    base, pattern = CACHE_GLOB
    for cache in sorted(base.glob(pattern)):
        if cache.name not in PROTECTED_CACHES:
            yield cache, 0
    if WORKTREE_ROOT.is_dir():
        for wt in sorted(WORKTREE_ROOT.iterdir()):
            target = wt / "target"
            if target.is_dir():
                yield target, MAX_SEARCH_DEPTH  # only the target itself
    base, pattern = TMP_GLOB
    for tmp in sorted(base.glob(pattern)):
        yield tmp, MAX_SEARCH_DEPTH - 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--apply", action="store_true", help="delete (default: report only)")
    ap.add_argument("--min-age", type=float, default=120.0, metavar="MINUTES",
                    help="skip directories modified more recently than this (default 120)")
    ap.add_argument("--keep", action="append", default=[], metavar="NAME",
                    help="cache directory name to leave alone (repeatable)")
    ap.add_argument("--only", metavar="DIR",
                    help="clean only this directory (a delegate's own cache or worktree); "
                         "use with --min-age 0 to clean up your own run immediately")
    args = ap.parse_args()

    cutoff = time.time() - args.min_age * 60
    keep = PROTECTED_CACHES | set(args.keep)
    total = removed = skipped = 0
    if args.only:
        only = Path(args.only).expanduser().resolve()
        if only.name in PROTECTED_CACHES:
            print(f"refusing to clean protected {only}", file=sys.stderr)
            return 2
        roots = [(only, 0)]
    else:
        roots = search_roots()
    for root, depth in roots:
        if any(part in keep for part in root.parts):
            continue
        for d in find_candidates(root, depth):
            if newest_mtime(d) > cutoff:
                skipped += 1
                print(f"skip (recent)  {d}")
                continue
            size = size_bytes(d)
            total += size
            kind = "target" if is_cargo_target(d) else "source export"
            print(f"{'remove' if args.apply else 'would remove'}  {size / BYTES_PER_GB:7.2f} GB  {kind:13}  {d}")
            if args.apply:
                shutil.rmtree(d, ignore_errors=True)
                removed += 1
    verb = "removed" if args.apply else "reclaimable"
    print(f"{verb}: {total / BYTES_PER_GB:.1f} GB in {removed if args.apply else '-'} dirs; skipped {skipped} recent")
    return 0


if __name__ == "__main__":
    sys.exit(main())
