# Monkey Man reference-environment census

Read-only. The parent runs this on Monkey Man. The script writes only under
the empty output directory it is given. OUT's parent must already exist.
OUT itself must be absent or empty, and it must not be a symlink. A symlink
anywhere in OUT's ancestry under
`/Users/marcelo/rustuml-reference/scratch` is also refused. The script
does not create ancestor directories, follow those links, install
packages, change Homebrew, or write anywhere else.

Copy this directory to Monkey Man unchanged. `reference_env_census.py` is
the checkout's `scripts/reference_env_census.py`. `census.txt` is the
committed census, sha256
`3e5d4793194c9f385b9bd3500a06789614ab4d705d7656c774f32df57322b1b2`
(relay `/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g7-census3/`,
2026-11-07). `reference_env_gate.py` is the hook. It is not part of the
record command below.

```sh
OUT=<empty directory whose parent already exists>
/opt/homebrew/bin/python3 -I reference_env_census.py record "$OUT"
/opt/homebrew/bin/python3 -I reference_env_census.py check "$OUT" --committed census.txt
```

`record` runs the reference JVM once with `DYLD_PRINT_LIBRARIES` and hashes
the loaded HarfBuzz and FreeType libraries, the JDK, the modelled face
files in `FONT_FILES`, and every other face named by a stored `#slot`
line (`CASCADE_FONT_FILES`). That list is derived from the font-chain
TSVs. Round-9 contributes 29 system faces, including Apple Color Emoji.
Round-6 contributes 12 more. It also hashes `~/Library/Fonts`,
`/Library/Fonts`, and every MobileAsset font file. An asset font is a
`.ttf`, `.otf`, `.ttc` or `.dfont`. Catalog `.xml` files and `Info.plist`
files are not asset fonts and are not written.

A user-font symlink is stored as the target file's size, the target
file's SHA256, the link path, and the resolved target path. A dangling
link is stored with sha256 `dangling`. The committed row for
`/Library/Fonts/Arial Unicode.ttf` is target size 23278008, sha256
`876af2cd4854644e7f3e7feb2f688997fdb3343c6df6693611209c9dfb47ccec`,
and target `/System/Library/Fonts/Supplemental/Arial Unicode.ttf`.
Do not `brew upgrade`.

This committed census has none of the cascade rows. `check` therefore
exits 1, and the DRIFT lines should be `extra font` for those paths
only. One of them is `/System/Library/Fonts/Apple Color Emoji.ttc`.
Return `$OUT/census.txt`. Do not treat any other drift as part of this
schema change.

Only exit 0 passes. Exit 1 means a pinned fact moved. Exit 2 means a
hash or a scalar is still unpinned. Both fail a capture.

Do not install these files on Monkey Man from this checkout. Against
this census the hook would abort every batch. The parent installs them
after the returned census is adopted and the independent re-review
accepts the packet.

Install, after that re-review, on Monkey Man:

- `/Users/marcelo/rustuml-reference/reference-env/reference_env_census.py`
- `/Users/marcelo/rustuml-reference/reference-env/reference_env_gate.py`
- `/Users/marcelo/rustuml-reference/reference-env/census.txt`

`/Users/marcelo/rustuml-reference/scratch` must already exist. In
`seq_capture.py`, immediately before `new-worker`, call
`require_reference_env()` as the hook's docstring shows. When the census
legitimately changes again, replace `census.txt` at that path with the
reviewed record. Replace the two scripts only when their source changes.
