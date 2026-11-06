# Checker measurements for 79f1104c7

Independent check of maker tip `79f1104c7f7570fa4026c8bcec26b0da695218f8` against parity-recovery base `cb550c0d67cc729ce764895005b7e86310c9f313`. Java pin `71806a23780b04a5ccde2f8ceb5121edad5eb711`. No Java was run on this Mac. The relay directories under `~/Library/Caches/rustuml-relay-parent/` were read and never written.

Hashes, the mutation matrix, and the private-cache log paths are in this directory. Full cargo logs stay in `~/Library/Caches/rustuml-grok-g7-checker/evidence/` and are not committed.

## Identities

| Item | SHA256 |
| --- | --- |
| `scripts/reference_env_census.py` | `48bd4273a7709c8d2f32de4b8bc7256babe54141497fa357270a68fe4f5fc991` |
| committed `census.txt` | `868a9072ecffd939c07e174b7c606679b5ac379a5ae7c1d48f2cf25336458ae6` (50752 bytes) |
| tip `rustuml` release | `112a82a4bebe6085d7d0a6e1b857732462e684ce876f803fde67e808c3d91ab2` |
| base `rustuml` release | `aa7c7790ddfa0dd029b0cd857ab961259a33ec9d6572bd6a09d8ebebd195eac4` |

`grok-g7-census2/census.txt` is byte-identical to the committed file. `check` of that directory exits 0.

Census rows: 226 `user-font`, 61 `asset-font`, 22 `font`, 24 scalar keys. One user-font row is `link` (`/Library/Fonts/Arial Unicode.ttf`, size 52). FreeType sha256 in the file is `1bf70387c4ef59accba8116c847cb87d2018ded2672db551686b95dce4c5d931`.

The stored historical `grok-g7-census/check-output.txt` has 64 `DRIFT` lines, all `.plist` (61) or `.xml` (3), plus 287 `UNPINNED`, and `check_exit=1`. That matches the maker's description of the old script's run. Checking that same old directory against the new committed census exits 1 with 59 `asset-font` drifts (the hashes the old record left as `-`). It is not metadata-only relative to the committed baseline.

## Mutation matrix

`python3 -I scripts/reference_env_census.py check LIVE --committed` the committed census. Copies only. Exit codes are in `mutation-matrix.txt`.

Exit 1, as required: HarfBuzz keg, HarfBuzz dylib hash, opt target, FreeType hash, JDK hash, JDK home, OS build, OS product version, user-font hash, user-font removed, user-font added, asset-font hash, asset-font removed, asset-font added, Helvetica.ttc hash, `/Library/Fonts/.uuid` hash, symlink size change, live census missing the FreeType key, old relay directory.

Exit 0, and these are not pinned-fact drift: identical copy, census2 directory, reordered asset rows, reordered scalar keys, comment header, catalog `.xml` and `Info.plist` rows even with a fake hash, extra live scalar, relay copy whose `SHA256SUMS` and `methodA.loaded.txt` pid were changed while `census.txt` was unchanged.

Exit 0 where the gate should have exited 1: a same-length symlink retarget, and an in-place change of the target file. `record` stores `(user-font, lstat size, "link", link path)` and does not store the target path or a hash of the target (`scripts/reference_env_census.py` lines 472–473). A tempfile probe with two 20-byte-name targets and different file bytes produced identical rows. A target one byte longer changes the size and exits 1. `/System/Library/Fonts/Supplemental/Arial Unicode.ttf` is not in `FONT_FILES`.

Exit 2 is not returned for this fully pinned baseline against a complete live copy. It is returned when the committed FreeType hash is blank, or when a committed asset hash is `-`, and the live side still has a real hash. Exit 2 means "not drift" in the exit status. A caller that treats only exit 1 as failure will accept an unpinned baseline.

## Read-only `record`

Python writes in `record`: `outdir.mkdir(parents=True, exist_ok=True)` at line 369, then `sw_vers.txt`, `static.txt`, `methodA.svg`, `methodA.stderr`, `methodA.exit`, `methodA.loaded.txt`, `census.txt`, and `SHA256SUMS`, all under OUT. `self-test` writes only inside `tempfile.TemporaryDirectory`. `assemble` writes the outfile the caller names. There is no `chdir`, no assignment to `os.environ`, and no `brew`.

The one child is `[REFERENCE, "--cwd", OUT, "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"]` with `env = os.environ.copy()` plus `DYLD_PRINT_LIBRARIES=1` (lines 395–403). `REFERENCE` is `/Users/marcelo/rustuml-reference/bin/reference`, which is not on this Mac. This checker did not run it. `seq_capture.py` reconstructs a headless wrapper and does not pass `-XX:-UsePerfData`, so a real HotSpot run can write `/tmp/hsperfdata_<user>`. That was not observed.

`mkdir(parents=True)` creates missing ancestors of OUT. That is a write outside OUT when a parent is absent. It does not write Homebrew kegs or font files.

`python3 -I` is what the script's docstring and the render test both use. This checker's `check` and `self-test` invocations used `-I`.

## Wiring

`~/.cache/rustuml-relay-tools/{upload.py,seq_capture.py,pull.py}` do not mention the census script. `seq_capture.py` validates the batch, then `new-worker`, then the reference JVM. Nothing in that file runs `reference_env_census.py`. `docs/reference-bootstrap.md` lines 196–197 call `check` "the drift gate a relay runs before trusting a new Java reference." That loop is not in the tools. The cargo test runs `self-test` on the committed file. It does not read Monkey Man.

Proposed enforcement, without this checker editing the tools: ship `reference_env_census.py` and the committed `census.txt` with the relay tools, and make the first step of `seq_capture.py` on Monkey Man, before `new-worker`, be `python3 -I reference_env_census.py record "$OUT" && python3 -I reference_env_census.py check "$OUT" --committed census.txt`, aborting the batch on exit 1 or exit 2. `upload.py` runs on the parent and cannot see Monkey Man's fonts except over SSH.

## Tests that read the census

`cargo test --release --offline -p rustuml-render --lib committed_census_matches_stored_face_extractions` exited 0. The test prints the self-test drift line and `self-test ok`. It compares committed rows to stored face-hash files and to the round-6 `ls` censuses. It needs no network and no Monkey Man. Workspace `cargo test --workspace --lib --release --offline` exited 0 (layout 41, math 10, oracle 31, parser 665, render 863, one ignored).

## PDF

Pinned `TitledDiagram.java` lines 336–346 comment out `exportDiagramNow`, including `FileFormat.PDF` at line 344. `TextBlockExporter12026.java:267` throws `UnsupportedOperationException` in the `default` arm. The CLI still defines `-tpdf`.

`cargo test -p rustuml --test pdf_contract` exited 0. Further tip-binary probes (sequence, class, activity, unicode, sprite, creole, a note with a 1×1 PNG): six pipe runs, one hash each; file mode matched pipe; cwd `/tmp` matched `/Users/marcelo`; three runs with `RUSTUML_DEBUG` unset matched the pinned-date runs; eight parallel sequence renders had one hash. Sequence PDF starts `%PDF-1.7`, length 2078, `startxref` 1817 lands on `xref`, one `/Type /Pages`.

`diff -rq` against `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/svg2pdf-0.13.0` shows content changes only in `src/util/context.rs` (sort fonts by reference before `write_font`) and `src/util/resources.rs` (sort resource entries by name), plus `RUSTUML-PATCH.md` which is only in the vendored tree. `LICENSE-APACHE`, `LICENSE-MIT`, and `NOTICE` are in `third_party/svg2pdf`. Repo `NOTICES` lines 32–37 already name svg2pdf (MIT/Apache-2.0, Typst). That file is not in the packet diff. PNG is resvg and EPS is the EPS writer. They do not call svg2pdf. The sample renders did not change.

## Non-PDF identity

Both release binaries, `RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100`.

| Format | Compared | Mismatches |
| --- | --- | --- |
| `-tsvg` | 18878 (12550 golden + 6328 perturbation) | 0 |
| `-tpng` | 64 (goldens step 400, perturbations step 200) | 0 |
| `-teps` | 64, same sample | 0 |
| `-ttxt` | 64, same sample | 0 |

`golden_no_oracle` on both trees: 11181/11251 eligible, same per-family lines, test ok. Failure-name file sha256 `4ddf284a7f9133f8bf094f56c32b21be349e41821d52d51cdcb3015eaa1b4221` (70 names), byte-identical.

`golden_pairs` on both: 12550 total, 10497 passed, 754 failed, 1299 skipped (1199 golden_error, 100 other). Failure-name sha256 `b92f592c7910f5fa5aaed1d6ddc6df7b7e2e7d79d1404c7e242091b06659ae1b`, byte-identical. The test binary exits non-zero because 754 pairs still differ from Java. That is the same at base.

`parity_perturbations` exits non-zero on both. The set of `test-diagrams/perturbations/*.puml` paths named in the two logs is equal (2727 paths, no side unique).

`git diff --name-only cb550c0d67cc729ce764895005b7e86310c9f313..79f1104c7 -- crates/rustuml-oracle` is empty. Decimal ratchet panic text matches: `class.rs` 383→578, `deployment.rs` 43→44, `plantuml_metrics.rs` 1053→1055, `state.rs` 53→62. Normalized `harness_guards` violation lines: 2304 and 2304, equal. After the base archive's empty golden directory was replaced with a symlink to the hydrated submodule, base `harness_guards` passes `error_golden_census_is_frozen` and still fails only `decimal_literal_ratchet` and `parity_review_records_are_complete`, the same two failures as the tip. Clippy `--release --offline --workspace --all-targets` exits 0 on both. Base emits 23 warnings. Tip emits those 23 plus two in `third_party/svg2pdf/src/util/helper.rs`, a file the packet does not modify.

`pdf_contract`, `pipe_mode`, and `doc_claims` on the tip exited 0.

## What this record does to the guard

`harness_guards` was run on the maker tip before this directory existed. It failed `decimal_literal_ratchet` and `parity_review_records_are_complete`, and passed the other four tests. This review's verdict is `rejected`. The guard requires the latest review to be `accepted`, so committing this record adds a violation. No product `.puml` files were added under `test-diagrams/perturbations/`. The `perturbations` array in `review.json` names paths that were deliberately not created, so the guard also reports those files missing. That is residue of recording a rejection of a non-diagram mechanism, not a product-corpus edit.
