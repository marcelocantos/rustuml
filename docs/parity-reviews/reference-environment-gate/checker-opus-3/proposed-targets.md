# Proposed targets

These are not written to `bullseye.yaml`. None of them blocks this packet.

1. **`font`-row hashes follow the unpinned rule.** In
   `scripts/reference_env_census.py check`, a committed `font` hash of
   blank or `-` should exit 2, as `user-font` and `asset-font` hashes and
   scalars already do. Today it exits 1 against a real live hash, and 0
   when the live hash is also blank or `-`. `record` cannot write such a
   live hash, and the Rust test rejects a short committed hash. Even so,
   the README's sentence "Exit 2 means a hash … is still blank or `-`" is
   false for `font` rows. Acceptance: the eight `font-hash-committed-*`
   cases in `checker-opus-3/mutation-matrix-check.tsv` exit 2, and a
   held-out case on a cascade row other than Apple Color Emoji also
   exits 2.

2. **The ancestry check uses resolved paths.** `prepare_record_dir` should
   refuse a symlink ancestor whose resolved location is under the resolved
   scratch. Today the check is lexical, so
   `<alias-of-scratch>/hop/out` writes through `scratch/hop` (row
   "scratch-reached-through-outside-alias" in `record-matrix.tsv`). The
   hook never builds such a path. Acceptance: that row and a `..` variant
   are refused, and every current row keeps its result.

3. **The scope of system faces is justified in the docs.**
   `docs/reference-bootstrap.md` should say why system faces with no
   `#slot` line need no hash. They sit on the sealed system volume, so
   they can change only with an OS update, and that moves
   `host.build_version`. It should also say that the corpus's
   `Courier`/`Times` names do not open the hidden `Courier.ttc` and
   `Times.ttc`: `courier` maps to `monospaced`, which is Menlo, and the
   Java goldens measure Menlo's advance. In addition, a capture batch that
   names a family outside the probed set (Arial, Courier New, Helvetica,
   Lucida Grande, Menlo, Times New Roman, Verdana, or the logical names)
   should be flagged, because no stored `#slot` line models its own
   cascade.

4. **Minor.** Have the hook set `sys.dont_write_bytecode`, or run with
   `PYTHONDONTWRITEBYTECODE=1`, so that loading `reference_env_gate.py`
   leaves no `__pycache__` in `/Users/marcelo/rustuml-reference/reference-env/`.
   Separately, test HotSpot's `/tmp/hsperfdata_<user>` write on Monkey
   Man, or disable it there with `-XX:-UsePerfData`.
