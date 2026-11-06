# Addendum: census-gate recheck rejected at `f34f37be`

Checker review `f34f37bed9691001ee1113656951511808e4955b` on
`review/grok-g7-recheck` rejects 🎯T14.6.8 again and leaves 🎯T14.34
accepted. This note is the maker's response. It contains no code change.
The rejected review is not copied into `docs/parity-reviews/`, because
that directory would fail `parity_review_records_are_complete` until a
later review accepts it.

Measured by the checker, and accepted here:

- Twenty-nine system fonts named by round-9 `#slot` lines are outside
  `FONT_FILES`, the user-font walk, and the MobileAsset walk. They are
  absent from `census.txt`. Editing one leaves `check` at exit 0.
  `checker-grok-2/unpinned-cascade-slots.txt` is that list. Apple Color
  Emoji is one of them.
- `prepare_record_dir` follows a symlink OUT. `record` then writes into
  the link target, outside the lexical path.
- A scalar whose value is `-` on both sides exits 0. A blank scalar
  already exits 2. A row hash of blank or `-` already exits 2.
- Merging this checkout does not arm the gate. `seq_capture.py` still
  does not call `require_reference_env()`.

The following commit will:

- pin every face the stored font-chain `#slot` lines name, as a `font`
  row of content hash plus path, deriving the path list from those
  lines rather than typing it;
- add a self-test mutation that changes one of those cascade faces and
  requires exit 1;
- refuse a symlink OUT, and refuse a symlink anywhere in OUT's ancestry
  under `scratch`;
- treat a committed scalar of blank or `-` as unpinned, exit 2, the
  same rule as an unpinned row hash;
- say in `docs/reference-bootstrap.md` that the parent arms the gate by
  installing the three files and calling `require_reference_env()`
  immediately before `new-worker`.

At the time of this note the committed census (`3e5d4793…`, relay
`grok-g7-census3`, 2026-11-07) did not contain the cascade rows.

## Adopted

The 2026-11-07 record from `grok-g7-census4`
(`284d5ef4cf8a43abb92fab2436d7348b2ab00f64d05dbf5679f430c442575fe9`)
replaces it and pins the 41 cascade faces. The hook is still not
installed from this checkout.
