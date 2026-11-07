# G7 recheck 3: independent checker measurements

Checker: Opus 5.5 (`claude-opus-5-5`), T3 thread t3-0f350db2, branch
`review/opus-g7-recheck3`, no child agents. Maker tip
`2ea341238b3f058e2088a3dddb8d48bf07ef51ee` (`maker/grok-g7-oracle-env-pdf`),
base parity-recovery `cb550c0d67cc729ce764895005b7e86310c9f313`, goldens
`9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`. Private cache
`~/Library/Caches/rustuml-opus-g7-recheck3/`. Java was not run. No file under
Monkey Man, `rustuml-relay-parent` or `~/.cache/rustuml-relay-tools` was
written.

Earlier rejections read with `git show`: `0c25fde68` (`review.json`) and
`f34f37bed` (`review-2.json`, `checker-grok-2/unpinned-cascade-slots.txt`).
They are not copied here. Their perturbation sources were never created, so
copying them would break `parity_review_records_are_complete`.

## 1. Coverage

**The stored chain data.** 35 TSVs carry `#slot` lines: 13 in round-6 and
22 in round-9. Between them they name 50 distinct paths. Every one is
pinned:

- 9 are `FONT_FILES`.
- 41 are `CASCADE_FONT_FILES`: 29 from round-9, plus 12 that only round-6
  names.
- The MobileAsset PingFang path is an `asset-font` row.

No `#slot` path is outside the census. All 29 paths in
`checker-grok-2/unpinned-cascade-slots.txt` are now `font` rows, as exact
byte-for-byte matches (`review2-unpinned-cascade-slots.txt`).
`census.txt` has 63 `font` rows (22 + 41), 226 `user-font` rows and 61
`asset-font` rows.

**What the corpus reaches.** The method is Java's own code, read from
JDK 21.0.13 `src.zip` (`jdk-source-excerpts.txt`). The reference JVM is
21.0.5. Java was not executed.

- *Font registration.* `CFontManager.getFontPath` returns `""` under
  `LWCToolkit`, so Java registers no font files. It registers only the
  fonts the native `loadNativeFonts` enumerates, plus logical fonts:
  - Serif maps to Times, then Times New Roman.
  - Monospaced maps to Menlo, then Courier.
  - Dialog and SansSerif map to Lucida Grande.
- *Cascades.* `CFont.createCompositeFont` uses
  `CTFontCopyDefaultCascadeListForLanguages` over
  `CFLocaleCopyISOLanguageCodes()`, then appends GeezaPro.
  `getOrCreateFallbackFont` drops dot-names such as `.TimesFB`, and
  `.AppleSymbolsFB` becomes AppleSymbols.
- *Compatibility names.* `SunFontManager.findFont2D` maps `courier` to
  `monospaced`, `timesroman` to `serif` and `helvetica` to `sansserif`. It
  does this only after normal lookup fails.

**Family names in the corpus.** The golden and perturbation corpus names 36
distinct families (`requested-families.txt`). On colossus (macOS 27.2,
26B123, the same build as Monkey Man), Core Text enumerates only Arial,
Courier New, Helvetica, Lucida Grande, Menlo, Times New Roman and Verdana
among them. Every file behind those families is pinned
(`requested-files-pin-status.txt`).

`Courier` and `Times` resolve by name to `/System/Library/Fonts/Courier.ttc`
and `Times.ttc` (`coretext-resolve.txt`). Neither file is pinned, and 30
corpus files name them (`courier-times-serif-fixtures.txt`). However, they
are hidden: `CTFontManagerCopyAvailableFontFamilyNames` lists neither, so
Java does not register them. Java's own output shows the result:

- `w_courier_pua_b_ctl.svg` measures `Vault` at 14 px as 42.1436.
  Menlo gives 42.14355, Courier.ttc 42.00684 and Lucida Grande 34.856.
- `alias_courier_upper.svg` measures `Wi mi` at 12 px as 36.123.
  Menlo gives 36.12305.

So `Courier` reaches the pinned Menlo through `monospaced`. Both pairs are
preserved under
`test-diagrams/perturbations/reference-environment-gate/` and pass. A
diagram naming `Times` or an unknown family falls back to Dialog, which is
Lucida Grande and pinned. `serif` resolves to Times New Roman, also pinned.

**Cascades for styles round-9 never probed.** I computed the cascade lists
locally with `cascade.swift`, which makes the same Core Text call as Java
with 631 languages, the same count as the stored `#cascade_languages`. The
local lists for Helvetica and Times New Roman match the stored slot names,
apart from the dot-names Java drops. Over 25 reachable faces (the 4 Times
New Roman styles plus every other style of the seven families) there are
50 paths. Only one is outside the census: colossus's Reserved
`PingFangUI.ttc`. On Monkey Man that slot is the pinned MobileAsset
PingFang, and losing that asset is a missing `asset-font` row, which is
exit 1.

**The documented scope.** The docs say "a face with no `#slot` line stays
outside the gate". Nothing the corpus reaches falls outside it. Any other
system face is on the sealed, read-only system volume (`df`, `mount`,
`diskutil`: `disk3s1s1`, sealed snapshot). Such a face can change only with
an OS update, which moves `host.build_version` and gives exit 1. All 63
`font` hashes are identical on colossus
(`colossus-vs-census4-font-rows.txt`). The docs do not spell out this
sealed-volume argument; see proposed target 3.

**The derivation.** It is deterministic: the same 41-tuple comes back under
`PYTHONHASHSEED` 0, 1, 2 and 12345 (prefix `2e1a66da06c251e4`). See
`stale-tuple-matrix.tsv`, 17 cases, all as expected:

- These fail `self-test`:
  - dropping, adding or swapping a tuple entry;
  - writing a Hiragino name in NFC;
  - adding a new `#slot` path;
  - removing every Songti slot or Apple Color Emoji;
  - moving a slot path into `/Library/Fonts` or onto a modelled face;
  - a relative slot path, or no `#slot` lines at all;
  - dropping Apple Color Emoji's census row.
- These pass `self-test`: reordering slot lines, renaming a TSV, and a
  `#slot` line in a non-TSV file.
- A stale census that lacks a non-Emoji cascade row (Songti) also passes
  `self-test`. The gate still fails closed, because `check` reports an
  extra `font` row (exit 1).

## 2. Mutation matrix

**`check`.** `mutation-matrix-check.tsv` has 117 cases; each one runs the
committed script as a subprocess. Exit counts: 17 exit 0, 63 exit 1,
37 exit 2.

*Every earlier case reproduces:*
- relay copy, comments, row reorder, scalar reorder, extra live scalar,
  catalog `.xml`/`Info.plist` rows;
- HarfBuzz keg, dylib hash, `opt_target`, loaded path, current version,
  link date;
- FreeType hash, `opt_target`, `link_current`, missing key;
- JDK hash and home, `libfontmanager` hash, wrapper hash;
- OS build, product version, hostname;
- user-font hash, size, add and remove; asset-font hash, add and remove;
- modelled face hash, add and remove;
- symlink size, target hash, target path, symlink becoming a regular file,
  dangling symlink;
- `.uuid`.

*New cascade cases, all exit 1:*
- a round-9 face hash, a round-6-only face hash, and an NFD Hiragino face
  hash;
- a cascade face removed, added, or renamed to NFC;
- live census4 against committed census3.

*The scalar grid.* For each committed value (real, blank, `-`, missing),
the live value is real, other, blank, `-` or missing:
- committed blank or `-`: exit 2 in all 10 cases, including dash on both
  sides and blank on both sides;
- committed real: matching exits 0, anything else exits 1;
- committed missing: exits 0, because an extra live key is metadata.
- A blank or missing committed `shaping.freetype.sha256` against a real
  live hash exits 2.
- Unpinned plus real drift reports exit 1.

*User-font, asset-font and symlink-row hash grids.* For each of the three
kinds, a committed blank or `-` exits 2 against every live value. A real
committed hash exits 0 only on a match.

*`font`-row hash grid.* A committed blank or `-` gives exit 1 against a
real live hash, and exit 0 when the live hash is equally blank or `-`.
The unpinned rule is not applied to `font` rows (finding 4). In practice
this is not reachable:
- `record` always writes `sha256_file()` for `font` rows, or crashes;
- the Rust test asserts that every committed `font` hash has 64
  characters.

**`record`.** `record-matrix.tsv` has 27 rows, all as expected. It runs the
real `record()`; only the JVM is stubbed, and the font roots are moved into
a sandbox. A Python audit hook records every write.

*Drift through a full record and check:*
- unchanged host: 0/0;
- cascade face edited: 0/1;
- cascade face removed: record exits 1;
- cascade face turned into a symlink to other bytes: 0/1;
- symlink to the same bytes: 0/0, since the content and the path are
  unchanged;
- dangling cascade symlink: record exits 1;
- modelled face edited: 0/1;
- user font added: 0/1; user font turned into a symlink: 0/1;
- asset font edited: 0/1; catalog files added: 0/0;
- JVM exit 1: record exits 1.

*OUT confinement:*
- These are refused with nothing written: a symlink OUT to an empty
  directory, a dangling symlink OUT, a symlink ancestor under scratch at
  one or two levels, a `..` path into a scratch symlink, scratch itself a
  symlink, a non-empty OUT, OUT as a regular file, and a missing parent
  (the parent is not created).
- These are accepted, and every write stays under `realpath(OUT)`: an
  existing empty OUT, a non-symlink ancestor outside scratch, a symlink
  ancestor outside scratch (by design), and an outside alias of scratch
  followed by a scratch symlink (finding 5).

## 3. Script version

`47e680592d6b…` (`0a36d0816`, the version that recorded census4) and the
committed `d4f9a562c8a3…` differ in one top-level function only,
`cascade_slot_mutation`, which is self-test code (`script-version-ast.txt`).
The other node pair differs only in its line number.

`record` and everything it calls are identical, `CASCADE_FONT_FILES`
included. With the JVM stubbed and JDK 21.0.13 substituted, the two
versions recorded colossus to byte-identical `census.txt` (`11dda102…`) and
identical sidecars. The audit hook saw exactly the same writes: one
`mkdir(OUT)` and 8 files under OUT.

Checking that colossus record against census4 gives no `font`-row drift at
all. The drift is only what is specific to colossus: hostname, JDK, Homebrew
FreeType, kegs, link date, 210 user fonts and 61 asset fonts
(`colossus-record-vs-census4.txt`).

So on an unchanged Monkey Man, a fresh record by the committed script
reproduces census4. That needs no request directory. The relay
`grok-g7-census4` matches both `SHA256SUMS` and `PARENT-SHA256SUMS`, it is
byte-identical to the committed census, and `check` of the relay exits 0.

## 4. Hook

See `hook-sim.txt`. In the copies, only path constants change: `INSTALL`,
`SCRATCH` and `PYTHON` in the gate; `REFERENCE`, `JDK_HOME` and `SCRATCH`
in the census script. The test python is `/opt/homebrew/bin/python3`.

| Case | Result |
| --- | --- |
| Both steps exit 0 | `RETURNED` |
| Check exit 1 (HarfBuzz hash) | `SystemExit`, status 1 |
| Check exit 2 (scalar `-`, blank build) | `SystemExit`, status 1 |
| Record exit 1 | `SystemExit`, status 1 |
| Extra committed font row | Exit 1 |
| Missing census | Abort |
| Missing scratch | Abort; scratch not created |
| Scratch is a file | Abort |
| Scratch is a symlink | Record refuses; abort; nothing left in the target |

No temporary directory is left in any case. Outside scratch, the only write
is Python's `__pycache__/reference_env_gate.cpython-315.pyc` in the install
directory, on the first load (finding 6).

**The documented paste.** I took the docstring call block without its
`new-worker` line and inserted it before line 68 of the live
`seq_capture.py` (`eaf70825…`), with 4-space indentation. The result is
`seq_capture.pasted.py`. With only `ROOT`, `JAR` and the gate path rewritten
in the simulation copy:

- gate exit 1, gate exit 2 and record exit 1 each stop `seq_capture`
  before `new-worker` runs (no sentinel);
- a passing gate lets `new-worker` run.

The install step is exact and sufficient.

## 5. Read-only safety of `record`

The only writes are:

- `mkdir(OUT)`, with `parents=False`, when OUT is absent;
- `sw_vers.txt`, `static.txt`, `methodA.svg`, `methodA.stderr`,
  `methodA.exit`, `methodA.loaded.txt`, `census.txt` and `SHA256SUMS`, all
  under OUT.

Subprocesses: `sw_vers`, `hostname`, `otool -L` (twice) and the reference
wrapper. A HotSpot `/tmp/hsperfdata_<user>` written by the real JVM is
still untested residue, as both earlier reviews said.

## 6. No loss

The release binaries are `rustuml-tip` (`79283b05…`) and `rustuml-base`
(`ce263565…`).

- `-tsvg`: stdout and exit status match on all 18878 sources (12550 golden
  and 6328 perturbation). 0 mismatches.
- `-tpng`, `-teps` and `-ttxt`: 64 evenly spaced sources each,
  0 mismatches.
- `-tpdf`, two runs of the same binary: the tip matches 64/64; base
  differs on 8/64, the instability the patch fixes.
- `pdf_contract`: 2 passed.
- The vendored `svg2pdf` differs from crates.io 0.13.0 only at the
  `context.rs` font sort, the `resources.rs` entry sort, and
  `RUSTUML-PATCH.md`.

## 7. Process

- Addendum `2315197ab` (ADDENDUM-2 only) is an ancestor of fix `0a36d0816`.
  Addendum `518f3caa2` is an ancestor of `d5c1ed3fb`.
- The adopt commit `2ea341238` rewrote ADDENDUM-2's status paragraph and
  appended an "Adopted" section. The planned-change list is unchanged.
- The diff against base for `crates/rustuml-oracle`, `test-diagrams` and
  `tests` is empty before this record. `GEOM_EPS`, syntax-error markers,
  the park list, the error census and the allow-lists are therefore
  unchanged.
- `harness_guards` fails the same two tests on both trees.
  - `decimal_literal_ratchet`: the same 4 deltas.
  - `parity_review_records_are_complete`.
  - The categorized violation lines are equal: 2711, sha256 `ac887e42…`.
- Clippy exits 0 for every crate on both trees. The tip adds only the
  2 `mismatched_lifetime_syntaxes` warnings in the unmodified vendored
  `third_party/svg2pdf/src/util/helper.rs`.

## Gates (once each, foreground)

| Gate | Tip | Base |
| --- | --- | --- |
| `golden_no_oracle` | exit 0; 70 failure names `4ddf284a…` | exit 0; identical |
| `golden_pairs` | 12550 / 10497 / 754 / 1299; names `b92f592c…` | identical |
| `parity_perturbations` | 2727 failing paths `fc5d0f7c…` | identical |
| `harness_guards` | 4 ok, 2 failed (above) | identical |
| `pipe_mode` | 17 passed | 17 passed |
| `doc_claims` | 2 passed | 2 passed |
| `pdf_contract` | 2 passed | n/a |
| Workspace lib | layout 41, math 10, oracle 31, parser 665, render 863 | render 862 (no census test) |
| Clippy per crate | exit 0 ×7 | exit 0 ×6 |

## After this record

I re-ran the two affected gates on the tip with this record in place.

- `harness_guards` still fails only `decimal_literal_ratchet` and
  `parity_review_records_are_complete`. Its categorized violation lines are
  unchanged: 2711, the same file. Nothing is reported for
  `reference-environment-gate`, because the account, review-3 and both
  perturbation pairs validate.
- `parity_perturbations` fails on the same 2727 paths. The two copied
  pairs are in the walk and pass.
