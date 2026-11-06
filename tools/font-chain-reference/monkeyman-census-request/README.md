# Monkey Man reference-environment census

Read-only. The parent runs this on Monkey Man. The script writes only under
the empty output directory it is given. OUT's parent must already exist.
OUT itself must be absent or empty. The script does not create ancestor
directories, install packages, change Homebrew, or write anywhere else.

Copy this directory to Monkey Man unchanged. `reference_env_census.py` is
the checkout's `scripts/reference_env_census.py`. `census.txt` is the
committed census, sha256
`868a9072ecffd939c07e174b7c606679b5ac379a5ae7c1d48f2cf25336458ae6`
(relay `/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g7-census2/`,
2026-11-06). `reference_env_gate.py` is the hook. It is not part of the
record command below.

```sh
OUT=<empty directory whose parent already exists>
/opt/homebrew/bin/python3 -I reference_env_census.py record "$OUT"
/opt/homebrew/bin/python3 -I reference_env_census.py check "$OUT" --committed census.txt
```

`record` runs the reference JVM once with `DYLD_PRINT_LIBRARIES` and hashes
the loaded HarfBuzz and FreeType libraries, the JDK, and the modelled face
files in `FONT_FILES`. Those are the faces the Rust font chain opens by
absolute path, not every face Core Text can open. It also hashes
`~/Library/Fonts`, `/Library/Fonts`, and every MobileAsset font file. An
asset font is a `.ttf`, `.otf`, `.ttc` or `.dfont`. Catalog `.xml` files and
`Info.plist` files are not asset fonts and are not written.

A user-font symlink is no longer stored as lstat size plus `link`. The row
is the target file's size, the target file's SHA256, the link path, and
the resolved target path. A dangling link is stored with sha256
`dangling`. The committed census still has the old row
`user-font  52  link  /Library/Fonts/Arial Unicode.ttf`, so this check
exits 1 on that row and is expected to. Return `$OUT/census.txt`. Do not
treat any other drift as part of the schema change. Do not `brew upgrade`.

Only exit 0 passes. Exit 1 means a pinned fact moved. Exit 2 means a hash
is still unpinned. Both fail a capture.

Do not install `reference_env_gate.py` into `seq_capture.py` until the
returned census has replaced the committed file. Against this census the
hook would abort every batch.

Install, after that replacement, on Monkey Man:

- `/Users/marcelo/rustuml-reference/reference-env/reference_env_census.py`
- `/Users/marcelo/rustuml-reference/reference-env/reference_env_gate.py`
- `/Users/marcelo/rustuml-reference/reference-env/census.txt`

`/Users/marcelo/rustuml-reference/scratch` must already exist. In
`seq_capture.py`, immediately before `new-worker`, call
`require_reference_env()` as the hook's docstring shows. When the census
legitimately changes again, replace `census.txt` at that path with the
reviewed record. Replace the two scripts only when their source changes.
