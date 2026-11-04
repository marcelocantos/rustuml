# Class quark, note URL and Fashion conversion: maker packet7

Maker evidence only; fresh independent review is required. No acceptance, review attestation or target change is authored. Every historical verdict remains unchanged.

Baseline `eb09cecbc90f5690c4a09bf49118f2feff8c1c12`; independent evidence `157cc571ce99714d41e28d41bd56084b53b8f36d` imported as `07d00af240c9938b63b7672d67f285a53a1d536d`. Production is `17a3705e17cc79640a1bcfea241b61f9b762d957`.

## Mechanisms and chronology

Only `crates/rustuml-parser/src/parse/class.rs` and `crates/rustuml-render/src/class.rs` changed. The existing AST represents the repaired objects without an AST schema change. `text_render.rs`, `parse/mod.rs`, preprocessing, shared style/metrics/fonts, layout/native, sequence, comparator, gates and baselines are unchanged.

Three schema-complete accounts were committed before any new draft production code, and the production commit references their exact hashes:

- `b3c510e605fb39393d61e0700ac47f1b256cbb49`: checked quark context and URL grammar. Java `CucaDiagram.quarkInContextSafe/badName` rejects repeated/trailing namespace separators, excludes the current group from unique reuse, and separates current-context declarations from reuse. Rust's root-entity shortcut merged native local declarations and rejected local named notes. The resolver now returns errors through the consuming commands; root/local data and UID events remain distinct. `CommandUrl`/`UrlBuilder` recognize `Pattern2 %g` quotations. `StringUtils.isDoubleQuote` cleans ASCII/curly target quotes but deliberately does not clean the internal Jaws delimiter. URL text is not globally rewritten.
- `d3dc83434df9ad36f7c0a505eb341b53807db1b6`: note URL lifecycle. Java `CommandFactoryNoteOnEntity.createSingleLine` passes null URL; the multiline factory constructs a URL. Rust assigns ownership during command consumption. `EntityImageNote.drawU` owns the anchor inside the entity group even under `EntityImageDegenerated`; the lone named-note renderer now consumes the retained URL and tooltip. No parser-owned anchor is hidden in rendering.
- `4bbe21b6ad4428b7dc1166f4081bed4729dfc29d`: description/note Fashion values. Java `Style.getShadowing` calls `ValueImpl.asDoubleDefaultTo(1.5)`, extracting ASCII digits/dots. Thus signed values lose their sign, `1e2` becomes 12, and empty/malformed extracted values use 1.5. Legacy boolean conversion remains distinct through existing declaration-origin APIs. Boundary subsequently normalizes nonzero shadow to 4; actor keeps the converted delta. No shared style module was changed.

The original packet6 schema and chronology defects are not repaired retroactively. Three original accounts omit `mechanism`; some corrective accounts followed draft experiments despite preceding the final implementation commit. Their original bytes, hashes and the admitted defect remain in `../../prior-chronology.json`. No historic account or review was normalized or rewritten. The archived integrity check verifies 8,802 prior evidence, perturbation, oracle and baseline files against their imported Git blobs. New account schema checks do not stand in for independent review; the evidence guard correctly still reports their absent reviews.

## CLI outcomes and denominators

Counts are unchanged-comparator matches / mismatches / valid-input process errors / invalid Java references. A comparator match is not byte-exact SVG equality.

| Scope | Entries | Before | After |
|---|---:|---:|---:|
| Preserved1559 | 1559 | 930 / 534 / 60 / 35 | 930 / 534 / 60 / 35 |
| Earlier maker36 | 36 | 32 / 4 / 0 / 0 | 32 / 4 / 0 / 0 |
| Earlier maker20 | 20 | 16 / 4 / 0 / 0 | 16 / 4 / 0 / 0 |
| Earlier checker34 | 34 | 26 / 4 / 0 / 4 | 26 / 4 / 0 / 4 |
| Preserved1649 | 1649 | 1004 / 546 / 60 / 39 | 1004 / 546 / 60 / 39 |
| Earlier maker26 | 26 | 20 / 6 / 0 / 0 | 20 / 6 / 0 / 0 |
| Imported checker86 | 86 | 36 / 17 / 2 / 31 | 41 / 14 / 0 / 31 |
| Frozen1761 | 1761 | 1060 / 569 / 62 / 70 | 1065 / 566 / 60 / 70 |
| Fresh32 | 32 | 7 / 16 / 3 / 6 | 20 / 6 / 0 / 6 |

1649 = 1559 + 36 + 20 + 34; 1761 = 1649 + 26 + 86. These constituent cohorts are disjoint by source hash. The 1559 contains 74 duplicate entries, so 1761 represents 1687 unique sources. Fresh32 is disjoint: combined 1793 entries represent 1719 unique sources. The six `retained_maker_` copies in the imported checker directory add no unique controls; the initial focused replay included them and is explicitly recorded as 92, not 86. Older 810/833/955/1192/1449 inventories overlap these cohorts and are not additional denominators. No newer parent aggregate is claimed. All replays use this worktree root as cwd, matching the reused baseline captures; the known T14.6.2 theme-include cwd dependence is not relabeled as a product geometry error.

Frozen gains: `classmode_actor_shadow_neg3`, `classmode_boundary_shadow_neg3`, `url_note_replace`, `url_root_class_ambiguous`, `url_smart_quotes`. `url_root_class_note_ambiguous` recovers parseability but remains a whole-SVG failure. Fresh gains and every residual, exit, stderr and source/reference hash are enumerated in `summary.json`, `all-case-outcomes.json` and `all-residuals.json`.

There are no lost passing comparisons, no new valid-input process errors, and no additional counted model-object losses across the tested before/after cases. This does not erase existing model defects. The 60 retained errors are 33 sequence panic entries and 27 parse-error entries, all preexisting; corrected TIM production is not integrated here. Invalid references never count as parity. Frozen invalid-input rejection improves 37/70 to 39/70; checker86 improves 28/31 to 30/31. Its remaining accepted invalid is unchanged `component_generic_contrast` (original missing-allowmixing source). Fresh negatives improve 1/6 to 6/6 rejected. Five were intended negatives; `quote_invisible` was intended positive but Java rejects it at target identity cleanup. Original sources, including missing-allowmixing cases, are retained unchanged.

## Model and DOM evidence

All 1793 sources were captured before/after through real CLI `--yaml`. 1772 YAML/status pairs are byte-unchanged; 21 changed pairs are enumerated, including seven newly rejected invalid inputs, five valid parse recoveries, two restored local entities and seven note-URL ownership changes. Successful preexisting Class models lose no counted entities, packages, notes, relationships or UID events. Explicit model checks verify complete root/local identities, owning package, target, URL and the creation-event stream; they do not infer retention from factory labels.

Root `Ledger` and local `Harbor.Ledger` receive separate Entity events around the Package event. A local NOTE beside a root class receives one Note event, retaining the root class. The recovered objects and their URL owners agree with Java SVG metadata, including IDs ent0002/ent0003/ent0004. All 47 bounded DOM observations are retained. Three root/local note cases still have a different serialization order despite matching objects/UIDs/URL owners. `url_rooted_existing` still lacks Java's `Basin.url` side-effect entity. No whole case is called repaired from an anchor-only or factory-only metric.

## Every fresh mismatch and causal scope

All six valid fresh failures are preserved as executable exact source/reference pairs with both Rust outputs, YAML and comparator diagnostics in `fresh32/`:

- `root_class_local_note`, `root_note_local_class`: models and UID/URL ownership agree; leaf serialization order differs. Java `GraphvizImageBuilder.printGroups/printGroup/printEntities` fills Bibliotekon with group leaves before unpackaged leaves, and `SvekResult.drawU` uses that order. Rust `class.rs::svek_node_emission_order` handles ordinary leaves by groups, but `render_plantuml_svg` interleaves notes by numeric UID. Group ownership and creation order are not the same as SVEK drawing order. This downstream ordering model remains open; no SVG reordering/postprocessing was added.
- `null_separator_lookup`: `CucaDiagram.quarkInContextSafe` null-separator firstWithName behavior, objects, UID and URL owners agree. Whole SVG differs only in viewport envelope: Java204x73 versus Rust202x71. The remaining path is Java `EntityImageEmptyPackage`/`SvekResult.calculateDimension` versus Rust `empty_package_dims` and final painted-envelope aggregation. The precise missing painted contributor is not proved here. No two-pixel compensation was introduced.
- `group_single`, `group_multi`: single-line URL absence and multiline URL ownership now agree, independently of geometry. Java cluster right edge143 versus Rust140 propagates to note text x175.98 versus172.98 and viewport291 versus288. Java `ClusterDotString.printInternal` point endpoints / `GraphvizImageBuilder` Opale eligibility feed compound-clipped SVEK edges; Rust `layout_package_clusters` and `add_svek_cluster_endpoint` consume upstream solved geometry. Prior exact-DOT point-node evidence is scoped to its captured environments. This remains an explicit graph/native/environment dependency, not a general environment excuse or permission to tune coordinates. The earlier `group_left/group_top` and all prior group-note failures remain whole-case failures after their URL defect is corrected.
- `actor_legacy_negative`: the actor's Fashion conversion now agrees; the companion native class still omits its filter. Java `EntityImageClass.drawInternal` reads numeric `Style.getShadowing` and sets the rectangle's delta. Rust `has_shadowing_skinparam` still recognizes only literal true for native class filtering, and `entity_painted_extent` assumes the enabled native delta3. Java92x211 versus Rust88x207 remains. This separate native-class style/painted-envelope dependency was not broadened into this description/note Fashion repair.

Preserved failures also include logical Serif metrics (`text_render::metric_family`), logical monospaced family/advance mapping, Q contour closure, cloud/nested-container/frontier/header/glyph differences and native styled-class generic metadata. `CommandLinkClass` precedes `CommandUrl` in Java factory ordering; the rooted `Basin.url` allocation/multi-pass side effect remains unproved and missing. No synthetic entity was invented. Historical size12 one-pixel canvas discrepancies remain in their original captures and are not called exact matches.

The read-only TIM handoff at `a7b3be50308f93ab2d97d6aee5fb9fbda522b7f2` reports exact evaluated source but truncated label/alias for `select_0/select_1` on its candidate. Its exact source/reference bytes and README are archived under `readonly-tim-handoff/`. Java `CommandCreateClass`/`NameAndCodeParser` owns the complete display/code command; this checkout's `name_token` still selects the first closing quote and `try_entity_decl` then validates the suffix. Corrected TIM plus this strict class parser is untested; no claim that the combined result matches or even has the same failure form. The other-family `o-->>o` factory conflict remains the supplied read-only handoff; no `parse/mod.rs` change or new factory claim was made.

## Final checks and failed iterations

All broad checks ran once on stable production `17a3705e17cc79640a1bcfea241b61f9b762d957`. Exact commands, cwd, environment, duration and real exits are in `corrected-gates.json` and the archive. Own target `/Users/marcelo/Library/Caches/rustuml-class-cff4/target`, `CARGO_BUILD_JOBS=4`, pinned stdlib, `RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100`.

- `cargo fmt --check`: pass.
- `cargo test --workspace --release --lib --no-fail-fast`: 1451 pass (42layout,10math,31oracle,587parser,781render).
- `cargo test --workspace --release --test pipe_mode --test error_paths --test doc_claims --no-fail-fast`: 13CLI +19error +2documentation pass.
- `cargo test --release --test golden_no_oracle -- --nocapture`: 11188/11251, exit101.
- `cargo test --release --test golden_pairs -- --nocapture`: 10523/11251 eligible, 728 failures and1299 skipped of12550, exit101. Both main failure-name files are byte-identical to packet6. These are library diagnostics, not CLI headline numbers.
- `cargo test --release --test harness_guards -- --nocapture`: 4/6, exit101; decimal and evidence guards remain red, including absent independent reviews for the new accounts. No allowlist/baseline changed.
- `cargo clippy --workspace --release -- -D warnings`: exit101,20 diagnostics at the preexisting sites.

Focused parser tests first passed96, then100; final includes101. Failed intermediate check captures remain: the first fallible-resolver draft missed `?` at one multiline call site; the initial internal-quote positive hypothesis caused a fresh invalid-input acceptance and one unit expectation failure. Source inspection of `StringUtils.isDoubleQuote` corrected that hypothesis before final gates. Both draft binaries' hashes, initial1761 replay,92-case focused replay, fresh before/initial/final outputs and build/test logs are retained. The final1761 replay was repeated only because production changed for that negative-input correction. No broad suite was rerun for evidence-only packaging. Sawmill's single-file parse failed with its not-a-directory scope-classifier error; the targeted edit used source inspection and compiler checks instead.

## Remote provenance and reproduction

Only the parent's existing Monkey Man relay generated new Java output. Input gzip SHA256 `4b0ef007882a94a64aa099dd82873e1d9374d346c18061c8a09cb6e1384e3bb0`; returned gzip `39b7ce1c19f901b67707d0a65861fd88906aad00c19d128f7852b5c2a3da6bfd`. Worker `/Users/marcelo/rustuml-reference/scratch/worker.JCe6mGHB`; explicit GMT+11:00; headless pinned JAR `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`, source `71806a23780b04a5ccde2f8ceb5121edad5eb711`, OpenJDK21.0.5 and Graphviz12.2.0. This is scoped calibration, not historical/global environment equivalence. All32 source/current-byte/stream hashes, wrapper/Java argv, cwd/TZ, gzip membership and no-timeout outcomes were independently revalidated locally. XML/error classification yields26 positives and6 negatives (exit0/200 respectively), not32 parity successes.

`packet7-evidence.tar.gz` contains all CLI/model/comparator outputs, failed iterations, full gate logs, input/reference content-addressed blobs, source audit, scripts and raw remote bundle. `input-reference-snapshot.json` maps each original path to immutable source/reference bytes; scripts retain the original private-cache argv for reproducibility. Rebuild the exact production commit into a private target, restore paths or adapt only capture destinations, and replay with the unchanged comparator (SHA256 `0e368102f36bfbc59efe141286df5935b785535db6c0cf537bdbadc9350f21cd`). `archive-integrity.json` records verification of every archived member. No local Java/javac, SSH, child agents, native cleanup/fault work, push, PR or release occurred. The packet stops for a new independent checker.
