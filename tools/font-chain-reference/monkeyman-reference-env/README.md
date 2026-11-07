# Monkey Man reference-environment census

Owner decision of 2026-11-06: Monkey Man's current fonts and Homebrew
HarfBuzz 15.0.0_1 / FreeType 2.15.0 are the Java oracle. Do not
`brew upgrade` on that host.

`census.txt` (sha256 `284d5ef4cf8a43abb92fab2436d7348b2ab00f64d05dbf5679f430c442575fe9`)
is the live record from
`/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g7-census4/`,
taken on 2026-11-07. The file in this directory is that record, unchanged.
It replaces the same day's `grok-g7-census3` record
(`3e5d4793194c9f385b9bd3500a06789614ab4d705d7656c774f32df57322b1b2`),
which had no cascade `font` rows. `methodA.loaded.txt` is the round-6
`DYLD_PRINT_LIBRARIES` extract
(sha256 `794020d8d86bae63a9f573bb2a0806de1c6e2cc9d7e787b04e4cbd80eed0ad38`).
The census4 capture also wrote a `methodA.loaded.txt`; that file is not
this extract and is not adopted.

FreeType 2.15.0, every modelled face, every cascade face named by a
stored `#slot` line, every regular user font and every MobileAsset font
file (`.ttf`, `.otf`, `.ttc`, `.dfont`) are content-hashed. Apple Color
Emoji is `12bc8221a908c5bcf05f0fce940d3c3f6e9d09d8d806ffe31a969b4ac9be8f9e`.
`/Library/Fonts/Arial Unicode.ttf` is a symlink. Its row pins target size
23278008, sha256 `876af2cd4854644e7f3e7feb2f688997fdb3343c6df6693611209c9dfb47ccec`,
the link path, and `/System/Library/Fonts/Supplemental/Arial Unicode.ttf`.
Round-6 `user-font-census.txt` still records the link length 52. That
listing is not re-extracted: `ls` does not hash the target, and this row
is the fact the gate checks. Catalog `.xml` and `Info.plist` files are
not asset fonts. Only `check` exit 0 passes.

`assemble` rebuilds the earlier artifact census from the stored captures,
including the round-6 lstat row. It is not how this baseline is refreshed.

The drift check is `reference_env_census.py check`. Only exit 0 passes.
Exit 1 means a pinned fact moved. Exit 2 means a hash or a scalar is
still blank or `-`, and that is a failed gate.
