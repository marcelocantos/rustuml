# Monkey Man reference-environment census

Owner decision of 2026-11-06: Monkey Man's current fonts and Homebrew
HarfBuzz 15.0.0_1 / FreeType 2.15.0 are the Java oracle. Do not
`brew upgrade` on that host.

`census.txt` (sha256 `868a9072ecffd939c07e174b7c606679b5ac379a5ae7c1d48f2cf25336458ae6`)
is the live record from
`/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g7-census2/`,
taken on 2026-11-06. The file in this directory is that record, unchanged.
`methodA.loaded.txt` is the round-6 `DYLD_PRINT_LIBRARIES` extract
(sha256 `794020d8d86bae63a9f573bb2a0806de1c6e2cc9d7e787b04e4cbd80eed0ad38`).

FreeType 2.15.0, every modelled face, every user font and every
MobileAsset font file (`.ttf`, `.otf`, `.ttc`, `.dfont`) are content-hashed.
One user-font row is a symlink and is marked `link`. Catalog `.xml` and
`Info.plist` files are not asset fonts.

`assemble` rebuilds the earlier artifact census from the stored captures.
It is not how this baseline is refreshed.

The drift check is `reference_env_census.py check`. Exit 0 means the pinned
facts match. Exit 2 means they match and a hash is still open. Exit 1 means
the reference host moved.
