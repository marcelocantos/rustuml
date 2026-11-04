# Packet8 maker results — pending independent review

This is implementation evidence, not a checker verdict or acceptance. Whole-output failures remain failures. No historical account/review, baseline, comparator, skip/allowlist, target or reference byte was changed.

## Commits and chronology

- Start: `154d4c7fd58bc7a1b91df7d01f530b3ea911a5fc`.
- Imported independent rejection `59d50dbc3da1d46f8a10ae703ae32d40c6e7a2ee` as `c51bb7686a96ac775d481273dfcf373402531524`. All three records, README and audit were read.
- Preimplementation account `fd342b277a7e22830c6b1a85d1a66529a40b658d`: `class-note-svek-paint-inventory`.
- Preimplementation account `2b349ba5c9869cdb7e3ed28e7cd761f87cb0c1db`: `class-attached-note-consuming-url-grammar`.
- Preimplementation account `da5395c5ddda94a716209c362d49a49b7dc1d87a`: `class-normalized-shadow-style-consumer`.
- Production: `90eab708a25bf80b86c4e44b571aa79e3f9b2d05`; its message references all three earlier account commits.

All three new accounts were committed before the first draft. Git proves commit ordering; the no-earlier-uncommitted-draft statement is maker provenance. Earlier missing-mechanism fields and corrective-account-after-draft defects remain recorded in the original blobs and `class-quark-context-checked-identity/prior-chronology.json`; this packet does not retroactively cure them. The integrity scan verifies 9,254 preexisting evidence/oracle/pair blobs unchanged.

The new accounts contain three inaccurate location qualifiers: NameAndCodeParser is in command/, StringUtils is at the PlantUML package root, and Boundary normalization is USymbolBoundary#getDrawing (with inherited asSmall), not getSkinParameter. `account-location-clarifications.json` records exact pinned paths/hashes. Original accounts remain unchanged; this is a postimplementation location clarification, not a retroactive preaccount claim.

Production changed only `crates/rustuml-parser/src/parse/class.rs` and `crates/rustuml-render/src/class.rs`. No `parse/mod.rs`, AST, text_render, creole, shared style/font/metrics, native/layout, TIM or sequence changes. No native fault work, replacement review, child, local JVM/javac, SSH, push, PR or release.

## Causal changes

1. `GraphvizImageBuilder.printGroups/printGroup/printEntities` inserts leaves into `Bibliotekon` by owner, with group leaves before root leaves. `SvekResult.drawU` uses that inventory. Rust now includes named/attached notes in `SvekEmissionOrder::collect_leaves`, ordered by actual parser UID events within each owner, and consumes that inventory directly while painting. It does not renumber UIDs or reorder serialized SVG. Existing distinct floating/attached graph-slot contracts remain; no solver adjustment was made. Actual insertion order also avoids treating repeated TIM source-line values as root entity creation order.

   `DotData.isDegeneratedWithFewEntities` requires zero groups and links. Both notes-only and single-native eligibility now include the complete group model. Empty groups reach the existing `EntityImageEmptyPackage` preferred sizing and primitive envelope path. This restores the dropped package in `q_null_note_first`; the older two-pixel `null_separator_lookup` difference also disappears by avoiding incorrect degenerated eligibility, without changing coordinates or dimensions.

2. `CommandFactoryNoteOnEntity` uses `NameAndCodeParser.codeForClass`, Pattern2 `%g`, CucaDiagram `cleanId` and strict `UrlBuilder`. Rust attached targets now consume ASCII/curly/Jaws quote tokens; identity cleanup removes ASCII/curly boundaries, preserving the Java-invalid Jaws target distinction. `parse_note_or_command_url` shares the existing five strict URL alternatives with standalone CommandUrl. Quoted spaces, tooltip and topurl survive. Both forms validate syntax; single-line passes null URL while braced/end-note forms retain it. Named-note inline URL syntax remains invalid. The actual pinned StringUtils file is `src/main/java/net/sourceforge/plantuml/StringUtils.java` (the account’s abbreviated utils/StringUtils identifies this class).

3. `FromSkinparamToStyle.convertNow` maps true/false for mapped keys; `getShadowingValue` maps yes/no only on unmapped global/note fallback. The existing shared Rust declaration conversion already retains that distinction in normalized values. `style_shadowing_delta` now applies `ValueImpl` digit/dot semantics directly to those values instead of converting them twice. Native `EntityImageClass` resolves numeric shadow through the entity class style; the renderer uses the same value for filter selection and `LimitFinder.drawRectangle` maxima. Preferred dimensions and the EntityImageDegenerated exception remain separate. Actor and boundary retain their distinct source-derived shadow semantics.

## Exact comparisons and model retention

| Cohort | Entries / unique sources | Before matches | After matches | After mismatches / errors / invalid refs |
|---|---:|---:|---:|---:|
| preserved1559 | 1559 / 1485 | 930 | 936 | 528 / 60 / 35 |
| maker36 | 36 / 36 | 32 | 32 | 4 / 0 / 0 |
| maker20 | 20 / 20 | 16 | 16 | 4 / 0 / 0 |
| checker34 | 34 / 34 | 26 | 26 | 4 / 0 / 4 |
| maker26 | 26 / 26 | 20 | 20 | 6 / 0 / 0 |
| checker86 | 86 / 86 | 41 | 42 | 13 / 0 / 31 |
| maker32 | 32 / 32 | 20 | 24 | 2 / 0 / 6 |
| checker33 | 33 / 33 | 18 | 25 | 2 / 0 / 6 |
| frozen1826 | 1826 / 1752 | 1103 | 1121 | 563 / 60 / 82 |
| fresh25 | 25 / 25 | 4 | 21 | 0 / 0 / 4 |
| combined1851 | 1851 / 1777 | 1107 | 1142 | 563 / 60 / 86 |

The cohort detail rows partition frozen1826: 1559+36+20+34+26+86+32+33. Frozen1826 has 74 duplicate entries; fresh25 is disjoint. Combined1851 has 1,777 unique sources. No entries were silently deduplicated, waived, or relabeled as positive matches. The first 810 positions are not claimed as the original810 inventory. This is the frozen class scope, not the current full parent corpus.

There are no lost CLI/comparator passes or new valid-input errors. Frozen errors fall 61→60; the remaining 33 sequence panic entries and 27 parse-error entries are preexisting. `all-case-outcomes.json` enumerates every status/exit before and after, and `all-residuals.json` enumerates every remaining mismatch, error and invalid reference. All 18 frozen recoveries and 17 fresh recoveries are individually listed in `summary.json`.

Frozen YAML/status bytes are unchanged for 1,824/1,826 entries. The two changes are `n_multiline_quoted_url` (correct URL string) and `q_curly_note_target` (valid model now parses). Fresh YAML/status bytes are unchanged for 18/25; changes include four valid parse recoveries, URL correction and the intended empty-quoted-URL negative changing from accepted to rejected. No counts drop on previously parsable valid inputs, and every such class UID event stream is unchanged. Complete object/owner/URL/UID inventories and raw YAML are preserved. All 48 current valid review/fresh DOM inventories agree in objects, UIDs, direct URL owners and paint order; this does not waive group geometry.

All four fresh negatives are Java error SVGs with raw exit200; Rust exits1. There are 21 valid Java class SVGs with exit0. No welcome/empty/malformed XML outputs occurred. The older82 invalid references remain separately counted, including31 still accepted by Rust; they are not positive parity evidence.

## Stable checks

All cargo commands use private target `/Users/marcelo/Library/Caches/rustuml-class-cff4/target` and `CARGO_BUILD_JOBS=4`. Commands, exact exits, binary/source hashes and environments are in `corrected-gates.json` and the archive. Final library/main/standing/guards/fmt/clippy checks ran once after production stabilized. Evidence-only work did not repeat them.

- `cargo test --workspace --release --lib --no-fail-fast`: 1,456 passed, exit0.
- `cargo test --workspace --release --test pipe_mode --test error_paths --test doc_claims --no-fail-fast`: 13+19+2=34 passed, exit0.
- `cargo test --release --test golden_no_oracle -- --nocapture`: 11,189/11,251, up from11,188; exit101 due frozen ratchet differences. No baseline edited.
- `cargo test --release --test golden_pairs -- --nocapture`: 10,525/11,251 eligible, up from10,523; 12,550 total,1,299 skipped,726 failures; exit101.
- Main failure-name sets show no newly failing cases. Product repairs `preprocessing/preproc_procedure_with_relationship`; assisted also repairs `preprocessing/preproc_complex_crud_generator`. Before/after failure-name files and set differences are retained.
- `cargo test --release --test parity_perturbations -- --nocapture`: 1,028/1,718 standing pairs; 690 failures including76 Java error references,33 panics,27 parse failures; gate exit101 (one metadata test passes). This differs from the CLI ledger because standing membership/cwd and invocation differ; no include or negative policy was changed.
- `cargo test --release --test harness_guards -- --nocapture`: 4/6, exit101. Decimal ratchet and historical/incomplete-review evidence guard remain red; the new accounts explicitly have no independent checker records.
- `cargo fmt --check`: exit0. `cargo clippy --workspace --release -- -D warnings`: exit101,20 existing diagnostics; no new diagnostic category/count.

Focused iteration retained compile failures (missing field/imported inventory variable/wrong Package field) and three initial new test failures from incorrect DOM selector expectations. Production focused65 improved38→49 matches. New unit assertions were corrected to real emitted UID/data-qualified-name/comment structure, not by changing production to match mistaken tests. `draft1.patch`, `draft2.patch`, check/build/unit logs, intermediate65/1826/25 captures and hashes remain archived. A final CLI replay after the Option-? cleanup and test-only selector corrections names the committed-tree binary; it reproduces the intermediate whole-SVG counts. No failed capture was overwritten.

## Residue and integration handoffs

- `n_group_single` and `n_group_multi` remain Java305x199 versus Rust301x199 with a different cluster right edge and connector. Previous maker32 `group_single`/`group_multi`, maker20’s four group note controls, checker34’s four group directions, and maker26 `group_left`/`group_top` remain whole-output failures. URL/UID/DOM correctness is not a solved geometry claim. Java SVEK cluster/Opale versus the Rust graph/native handoff remains an independent dependency; no environment compensation was attempted.
- Maker36 `header_0_15`, `header_0_9`, `header_4_15`, `header_4_9`; maker26 `actor_blank`, `actor_stereo`, `boundary_blank`, `boundary_stereo`; logical Serif/monospaced shared metrics, Q contour closure, cloud/nested group/frontier, native/custom header, and other original retained failures remain enumerated. The earlier tolerated size12 one-pixel canvas difference is not promoted to exact match.
- Rooted CommandLinkClass multi-pass allocation, other-family decorated-message/o-->>o viability, shared sequence font and excluded native point-layout/environment differences remain dependencies. No unaccepted sequence/TIM production was imported.
- TIM8 exact evaluated source `empty_include` retains Class `source_line:0`; current class renderer `entity_dims` uses index+1 for zero (native around1513 and description around1123). This is next-packet source metadata work, not a preprocessing fix. `readonly-tim8-handoff` preserves the exact committed reviewer README and hash from0f00130a26b2f300996d0bd427f9e70b4ef10e30. Welcome factory and unresolved sprite remain separate.
- TIM `select_0`/`select_1` nested-quote label/alias loss remains a combined integration handoff. Shared sprite worker reports `markup_literal`/`markup_scan` need a class generic AST/box as well as StripeSimple absent-sprite/flush/empty-atom behavior. Its parent-side id-only generic trace differs from this unintegrated class branch, which already tries split_generic(label) at parse/class.rs:993. That revision difference was reported to the parent; no combined result is claimed and no new production or replay was added.

## Reference and archive provenance

Java source71806a23780b04a5ccde2f8ceb5121edad5eb711; pinned JAR SHA256 aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053. All older reference bytes are reused unchanged. New references came only through the parent Monkey Man relay, private worker `/Users/marcelo/rustuml-reference/scratch/worker.5dKY3ZXw`, OpenJDK21.0.5, Graphviz12.2.0, explicit GMT+11:00 and headless wrapper. Those environment versions differ from origin; scoped calibration is not global historical equivalence.

Input gzip SHA256 `c5e86ef61319b3c52bf50b4f0c7fa7f80fc2441a89f8bb854576f76350062deb`; returned gzip SHA256 `60e4254f5edb2804026c4c0d846ce1283c2bbf126d5c4b5075fb04c9f12be5c0`. Maker independently verified membership/current bytes, source/stream hashes, gzip/base64, pinned JAR, requested/actual wrapper+Java argv, private cwd/timezone/empty overrides and no timeouts. Raw returned bundle, manifest and parent validation are retained unmodified. Exit status and XML content are classified independently.

The archive includes exact content-addressed source/reference snapshots for all1,851 entries, raw CLI/AST/YAML/comparator stdout/stderr and actual exit ledgers, scripts, failed drafts, gate logs, source/binary hashes and transport. Absolute paths record actual invocations; reproduction elsewhere requires explicit path remapping to the snapshot, never source trimming or fixture rewriting. Executables/targets are excluded but rebuild hashes/revisions are recorded. `archive-integrity.json` verifies each archived member. Fresh25 pairs and before/after SVG/YAML are directly accessible alongside this report.

Packet is committed locally for a new independent checker. It does not certify acceptance of these mechanisms or the earlier recovery campaign.
