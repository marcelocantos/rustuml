# Monkey Man reference-environment census

Read-only. The parent runs this on Monkey Man. The script writes only under
the empty output directory it is given. It does not install packages, change
Homebrew, or write anywhere else.

Copy this directory to Monkey Man unchanged. `reference_env_census.py` and
`census.txt` are the checkout's `scripts/reference_env_census.py` and
`tools/font-chain-reference/monkeyman-reference-env/census.txt`.

```sh
OUT=<empty directory>
python3 -I reference_env_census.py record "$OUT"
python3 -I reference_env_census.py check "$OUT" --committed census.txt
```

`record` runs the reference JVM once with `DYLD_PRINT_LIBRARIES` and hashes
the loaded HarfBuzz and FreeType libraries, the JDK, the modelled face files,
`~/Library/Fonts`, `/Library/Fonts`, and every MobileAsset font file. An
asset font is a `.ttf`, `.otf`, `.ttc` or `.dfont`. Catalog `.xml` files and
`Info.plist` files are not asset fonts and are not written.

`check` uses that same rule, so a census that still lists those catalog
files does not count them as drift. The committed census is the 2026-11-06
live record (`868a9072ecffd939c07e174b7c606679b5ac379a5ae7c1d48f2cf25336458ae6`,
relay `/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g7-census2/`).
Exit 0 means every pinned fact matches. Exit 1 means a pinned path, size
or hash moved. Exit 2 means the pinned facts match but a hash is still
open. Exit 2 is not drift. A matching host exits 0.
Return `$OUT/census.txt`. Do not `brew upgrade` on this host.
