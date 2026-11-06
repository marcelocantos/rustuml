# Recheck measurements

Verdict: rejected. Not accept-ready for a local merge.

Two reject conditions fired. Twenty-nine cascade system faces stay unpinned, so `check` exits 0 and, once the hook is pasted, the capture still proceeds. `record` on a symlink OUT writes into the symlink target, outside the lexical path. Non-PDF stdout did not change. A missing parent of OUT is not created.

No Java was run. The real `REFERENCE` path was not run. The relay parent was read and then copied; the copy was chmod 0644 so the matrix could write. The parent file stayed mode `r--r--r--`.

## Identities

| Item | Value |
| --- | --- |
| Review branch | `review/grok-g7-recheck` |
| Maker tip | `a2c9e7e674a2d0c1532446025310d994e061d180` |
| Base | `cb550c0d67cc729ce764895005b7e86310c9f313` |
| Prior rejection | `0c25fde686886786a34ceca5096687ec47bdf19f` on `review/grok-g7-env-pdf` |
| Sound PDF commit | `79f1104c7f7570fa4026c8bcec26b0da695218f8` |
| Addendum | `518f3caa2448b6e853ff12549236956158bc01a1` (ancestor of the fix) |
| Fix | `d5c1ed3fb2297ff5b858f8b95f60b858034a26f5` |
| Adopted census | `a2c9e7e674a2d0c1532446025310d994e061d180` |
| Golden gitlink, base and tip | `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a` |
| Java pin named by the account | `71806a23780b04a5ccde2f8ceb5121edad5eb711` |
| Census script | `112bb35f9a2f3693e7f70efbf5696d4be59edf11e61105af2f55b79f33e682d4` |
| Gate script | `7814afd02ffc30167aa469d0e9ad11f4db67b78aadf07cb084f8404de192353b` |
| Committed census | `3e5d4793194c9f385b9bd3500a06789614ab4d705d7656c774f32df57322b1b2` (50871 bytes) |
| Relay census | same hash, `/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g7-census3/census.txt` |
| Census rows | font 22, user-font 226, asset-font 61 |
| `FONT_FILES` | 22 paths, `scripts/reference_env_census.py` lines 56–79 |
| Tip binary | `f4e4f291a5adc1ca78048ba71e27ee9dbd5aaa7a7c4e5fe72853d8c9509b2584` |
| Base binary | `aa9598c5ce36a7ce3060e61d864a4a67e026f5657b2c62ae1b435628f76d72cb` |
| Live `seq_capture.py` | `eaf70825a4a05e6bad0d2c65bfd3ba781bc7da65ab443e10594537ae7549bb3c` |
| No-oracle baseline | `36dcc0f2c5eb3fe6906290dfcc3ffb7a8f2061b41d9ca7023c3df39566f406f0` |
| Private cache | `~/Library/Caches/rustuml-grok-g7-recheck/` |

PDF blobs are identical at `79f1104c7` and at the tip:

| File | Git blob |
| --- | --- |
| `crates/rustuml/tests/pdf_contract.rs` | `367f92395c3e982fd81ae5ee4f21b31a914cde53` |
| `crates/rustuml/src/main.rs` | `7c8b697a187c5f6513b8ea3803f5ac6f67c5d323` |
| `third_party/svg2pdf/src/util/context.rs` | `98f528ae85a1f4dc815ebed74defc5f551ab80d6` |
| `third_party/svg2pdf/src/util/resources.rs` | `36f79bcbb935cc5c4375838f87e6fcf0a07389e8` |
| `third_party/svg2pdf/src/util/helper.rs` | `b28d053e11f79a8f109e8cd54d8be6430fc9bbcf` |
| `third_party/svg2pdf/RUSTUML-PATCH.md` | `0fdfa168b408713042ea7f52ab861e5f5b12018f` |

`git diff 79f1104c7..HEAD` for those paths is empty. The tip-versus-base product diff that is not the already-reviewed PDF vendor is `crates/rustuml-render/src/lib.rs` (the `#[cfg(test)]` module line), `crates/rustuml-render/src/reference_env_census.rs`, `crates/rustuml/src/main.rs` (three help-text lines), the census script, the gate script, and `docs/reference-bootstrap.md`. `crates/rustuml-oracle/tests/harness_guards.rs` has no diff against the base. The oracle crate has no diff against the base.

## Commands

Goldens:

```bash
git -c protocol.file.allow=always submodule update --init \
  --reference /Users/marcelo/work/github.com/marcelocantos/rustuml/test-diagrams/golden \
  test-diagrams/golden
```

Tip tests, `CARGO_TARGET_DIR=~/Library/Caches/rustuml-grok-g7-recheck/target-tip`, offline, release. Log `logs/tip-tests.log`. Started 2026-11-06T22:25:25Z, `TIP_DONE` 2026-11-06T22:45:34Z. The target directory was deleted after `bin/rustuml-tip` was copied.

```bash
cargo test --release --offline --workspace --lib --no-fail-fast
cargo test --release --offline -p rustuml --test pdf_contract --test pipe_mode
cargo test --release --offline -p rustuml-oracle \
  --test doc_claims --test harness_guards --test golden_no_oracle \
  --test golden_pairs --test parity_perturbations --no-fail-fast
cargo clippy --release --offline -p <crate> --all-targets --message-format=short
```

Base tests used `git archive cb550c0d67cc729ce764895005b7e86310c9f313` at `$CACHE/base-src`, with `test-diagrams/golden` symlinked to the hydrated worktree golden. `CARGO_TARGET_DIR=$CACHE/target-base`. `BASE_DONE` 2026-11-06T22:53:13Z. The archive and the target directory were deleted after `bin/rustuml-base` was copied. Base did not run `pdf_contract`, `pipe_mode`, or `doc_claims`; those tests are not in that tree.

Byte compare, no Java, `RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100`:

```bash
python3 -I "$CACHE/evidence/compare_outputs.py"
```

## Gates

| Gate | Tip | Base |
| --- | --- | --- |
| Workspace lib | exit 0. layout 41, math 10, oracle 31, parser 665, render 863 passed / 1 ignored | exit 0. Same except render 862 passed / 1 ignored |
| `pdf_contract` | 2 passed, 0.77s | not in the base tree |
| `pipe_mode` | 17 passed, 310.94s | not in the base tree |
| `doc_claims` | 2 passed, 0.12s | not run |
| `golden_no_oracle` | passed, 14.40s | passed, 4.69s |
| `golden_pairs` | 12550 total, 10497 passed, 754 failed, 1299 skipped | same counts |
| `parity_perturbations` | 1 passed, 1 failed, 26.25s; 2727 failure paths | 1 passed, 1 failed, 12.27s; same 2727 paths |
| `harness_guards` | 4 passed, 2 failed | 4 passed, 2 failed |
| Clippy, six crates | exit 0 | exit 0 |

`golden_no_oracle` passing means the family counts match `no_oracle_baseline.txt`: 11181 pass, 11251 eligible. Cargo hid the family lines on success. The written failure-name files are the diff: both no-oracle files sha256 `4ddf284a7f9133f8bf094f56c32b21be349e41821d52d51cdcb3015eaa1b4221` (70 lines); both golden-pair files sha256 `b92f592c7910f5fa5aaed1d6ddc6df7b7e2e7d79d1404c7e242091b06659ae1b` (754 lines). `cmp` was clean.

Decimal ratchet text is the same on both trees: `class.rs` 383 to 578, `deployment.rs` 43 to 44, `plantuml_metrics.rs` 1053 to 1055, `state.rs` 53 to 62. Harness violation lines, after stripping the checkout prefix, are 2704 and 2704 and equal. The `docs/parity-reviews/` subset is 2300 and 2300 and equal. This was measured before `review-2.json` existed.

Clippy summaries that already exist on the base: `rustuml-parser` lib 1, `rustuml-render` lib 19 (17 suggestions), `rustuml-layout` lib test 1, `rustuml-oracle` lib 1 and lib test 1 duplicate, `rustuml-parser` lib test 1 duplicate, `rustuml-render` lib test 20 (19 duplicates). The tip also prints `svg2pdf` lib generated 2 warnings. Those are in unmodified `third_party/svg2pdf/src/util/helper.rs`.

## Output compare

`compare-outputs.log` ended:

```text
svg sources 18878 golden 12550 pert 6328
SVG_MISMATCHES 0 of 18878
sample 64
-tpng mismatches 0 of 64
-teps mismatches 0 of 64
-ttxt mismatches 0 of 64
EXIT:0
```

The sample is `golden[::400] + perturbations[::200]`.

## Mutation matrix

Independent `check` on copies. Exit codes are in `mutation-matrix.txt`.

Exit 0: relay copy, relay metadata-only sidecar, identical text, reordered rows, comment-only, extra live scalar, catalog xml/plist, link-to-link that resolves to the same file, relative spelling of the same target.

Exit 1: HarfBuzz keg, dylib hash, and opt target; FreeType hash; JDK hash and home; OS build and product; user-font hash, remove, and add; asset-font hash, remove, and add; font-face hash (Helvetica.ttc), remove, and add (Apple Color Emoji as an extra `font` row); symlink size, hash, and resolved path; uuid hash; live census missing the FreeType key; committed scalar `-` against a live real hash; same-length retarget; same bytes at another path; target-content edit; link replaced by a regular file; dangling; chain to a different file; relative link outside; link to a directory outside the census; symlink loop.

Exit 2: blank committed FreeType scalar, committed asset hash `-`, blank committed user-font hash.

Separate probes on the committed text, same function:

| Case | Exit |
| --- | --- |
| scalar `-` versus scalar `-` | 0 |
| scalar blank versus scalar blank | 2 |
| row hash `-` versus row hash `-` | 2 |
| row hash blank versus row hash blank | 2 |
| committed scalar `-`, live real hash | 1 |
| `self-test` | 0 |

`record` always writes a real digest, so the hook against this baseline does not take the both-dash path. `check` still accepts it.

## Hook and record

`checker-grok-2/hook-and-record.txt`. The gate script was copied and its three path constants were pointed at a fake install under the private cache. The census script in that install was a stub controlled by `GATE_RECORD` and `GATE_CHECK`. The stub's `GATE_ESCAPE` write is the stub, not `record`.

| Case | Process status | Sentinel `new-worker-ran` | Temp dirs left |
| --- | --- | --- | --- |
| check exits 1 | 1 | no | none |
| check exits 2 | 1 | no | none |
| record exits 1 | 1 | no | none |
| both exit 0 | 0 | yes | none |
| scratch directory absent | 1, and the directory was not created | | |

`prepare_record_dir` on the real module:

- Missing parent: `SystemExit`, parent still absent.
- Non-empty OUT: refused, the existing file kept.
- Fresh OUT: created with `mkdir(parents=False)`. A second `prepare_record_dir` on that still-empty directory was accepted and left it empty.
- Symlink OUT: `prepare_record_dir` accepted it (`exists` and `is_dir` follow links).

`record` on a real directory, with the reference JVM stubbed and not executed, wrote these files under that directory: `SHA256SUMS`, `census.txt`, `methodA.exit`, `methodA.jvm-not-run`, `methodA.loaded.txt`, `methodA.stderr`, `methodA.svg`, `static.txt`, `sw_vers.txt`.

`record` on a symlink to an empty directory outside the parent, with only that JVM call stubbed, returned `SystemExit` (`DYLD_PRINT_LIBRARIES showed [] for libharfbuzz.0.dylib`). Before the abort it had written `methodA.exit`, `methodA.loaded.txt`, `methodA.stderr`, `methodA.svg`, `static.txt`, and `sw_vers.txt` into the symlink target. The parent contained the symlink and no other entry. The `builtins.open` counter stayed at 0 because Python 3.13 `pathlib` uses `io.open`; the directory listing is the evidence.

## Round-6 sizes

`committed_census_matches_stored_face_extractions` passed as part of the 863 render tests. It drops Arial Unicode from the round-6 size map (ls size 52), asserts the census size 23278008, the target sha256 `876af2cd4854644e7f3e7feb2f688997fdb3343c6df6693611209c9dfb47ccec`, five fields, and the Supplemental path, then asserts the remaining user-font map and the asset map are equal. The round-6 file is an `ls` listing. It does not hash the target, and replacing it would drop the link length 52. The census row is the fact `check` uses.

## What this record does to the guard

`parity_review_records_are_complete` already fails on both trees. After this commit it also fails because `review-2.json` is the latest review, its verdict is `rejected`, and the two perturbation paths above were not created as paired `.puml`/`.svg` files. They were not created because a new product fixture would change the perturbation walk. The guard was identical to the base before this record.
