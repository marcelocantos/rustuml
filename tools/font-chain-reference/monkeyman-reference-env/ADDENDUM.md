# Addendum: census gate rejected at `0c25fde68`

Checker review `0c25fde686886786a34ceca5096687ec47bdf19f` on
`review/grok-g7-env-pdf` rejects 🎯T14.6.8 and accepts 🎯T14.34.
This note is the maker's response. It contains no code change.
The rejected review is not copied into `docs/parity-reviews/`, because
that directory would fail `parity_review_records_are_complete` until a
later review accepts it.

Measured by the checker, and accepted here:

- `record` stores `/Library/Fonts/Arial Unicode.ttf` as lstat size `52`
  plus the marker `link`. A same-length retarget and an edit of
  `/System/Library/Fonts/Supplemental/Arial Unicode.ttf` both leave the
  row unchanged, so `check` exits 0. That target is not in `FONT_FILES`.
- `record` calls `mkdir(parents=True)`, which creates missing ancestors
  of OUT.
- Exit 2 is still returned when a committed hash is blank or `-`.
  Wording that calls exit 2 "not drift" lets a caller treat it as success.
- `seq_capture.py` under the parent's relay tools never runs the census.
  Those tools are not edited from this checkout.

The following commit will:

- store a user-font symlink as the target file's size, the target
  file's SHA256, the link path, and the resolved target path;
- make a same-length retarget, a target-content edit, a link replaced
  by a regular file, and a dangling link each exit 1;
- leave `FONT_FILES` as the modelled faces the Rust font chain opens
  by absolute path, and pin every other user-font symlink on its own
  row instead of copying its target into `FONT_FILES`;
- refuse `record` unless OUT's parent already exists, and require OUT
  itself to be absent or empty;
- state one consumer rule: only exit 0 passes. Exit 1 and exit 2 both
  fail the gate;
- add `scripts/reference_env_gate.py` for the parent to call from
  `seq_capture.py` immediately before `new-worker`.

The committed `census.txt` at the time of this note (`868a9072…`, relay
`grok-g7-census2`, 2026-11-06) still had the old `link` marker. The
2026-11-07 record from `grok-g7-census3`
(`3e5d4793194c9f385b9bd3500a06789614ab4d705d7656c774f32df57322b1b2`)
replaces it. The Arial Unicode row is the target file. The hook is
still not installed from this checkout.

PDF and the non-PDF outputs are not part of this fix. The checker
already measured them as unchanged.
