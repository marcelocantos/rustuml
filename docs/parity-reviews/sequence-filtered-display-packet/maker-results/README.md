# Filtered sequence Display and note consumers: maker packet

This is implementation evidence, not acceptance or self-review. A NEW independent checker is required. Previous accepted/bounded records remain tied to their original tested revisions. Imported rejection `bd40c0f16838359779e406592edb51f32be2e9bd` unchanged as `7dcfb706517ed2a4af4601352e43471f45ae4973`.

Baseline: `0cde5d7e55139b63cb8e91f8e42acd473441f80a`. Full final tested production: `61e6aeed6eaf686b3497fbc696aadc03a64e69a2`. All9,145 prior account/review/pair paths and205 imported paths remain byte-exact. The changed existing files are only diagram/sequence.rs, parse/sequence.rs, render/sequence.rs and the three explicitly authorized test-constructor defaults in ascii.rs. Exact authorization/diff: [protected-evidence.json](protected-evidence.json), [ascii-authorized.diff](ascii-authorized.diff).

| Mechanism | Preimplementation account commit | Implementation commit |
| --- | --- | --- |
| sequence-filtered-ordered-participant-display | `ca2aae93f8056b9ff0a93e08cd6bafda7be2360a` | `7c05cfcf8cc65be31e2f69d68bf3ee432532f9a1` |
| sequence-note-aggregate-paint-height | `ca2aae93f8056b9ff0a93e08cd6bafda7be2360a` | `fc71fafc7f372be708ec077dba7d82748ba7667a` |
| sequence-external-note-owner | `ca2aae93f8056b9ff0a93e08cd6bafda7be2360a` | `aa5ac30e6660b2623a8ceb8fd1e72c3108c8458c` |
| sequence-note-shape-terminator | `5d4858fb269c68ec8e6ec7261f41c9c84a7a4618` | `61e6aeed6eaf686b3497fbc696aadc03a64e69a2` |

Participant Display keeps the raw stereotype, captures declaration-time bottom order and the final diagram-wide hide state. Both new fields default on deserialization and are omitted when false. Visibility filters before all participant component metrics; queue initial/created/tail consumers share the ordered Display for measure and paint. Spotted/multiple/stereotype-style composition remains unresolved.

Rectangular and hexagonal notes now cast the complete text-block height plus their constructor marginY4 twice, once. Folded marginY5 aggregation was already correct and is preserved. The fresh immutable Java-valid `end rnote`/`end hnote` controls exposed a parser rejection, retained in candidate1 evidence. A separately precommitted account derives standalone end-command grammar from the Java factories; attached arrow-note terminators remain distinct. External messages now set the bare-note owner from normalized endpoints. This retains labels/events without claiming all external-note geometry is repaired.

| Real CLI cohort | Entries | Before match / SVG mismatch / valid error / invalid reference | After |
| --- | ---: | --- | --- |
| Frozen | 2281 | 1611 / 593 / 0 / 77 | 1611 / 593 / 0 / 77 |
| Imported checker | 64 | 33 / 27 / 2 / 2 | 41 / 19 / 2 / 2 |
| Fresh relay | 24 | 4 / 16 / 4 / 0 | 22 / 2 / 0 / 0 |
| Exact Creole handoffs | 2 | 0 / 2 / 0 / 0 | 0 / 2 / 0 / 0 |
| Total | 2371 | 1648 / 638 / 6 / 79 | 1674 / 616 / 2 / 79 |

26 recoveries, zero lost passes in CLI/product-library/assisted-library comparisons, zero new valid CLI errors, zero SVG render panics. The four introduced hidden-wide-name losses are repaired; their head/label/shape/height evidence is in creation-witnesses.json.gz. Semantic inventory records24 changed geometry cases and20 changed count cases; no new/larger excess text/element counts against Java. Counts are diagnostics, not whole-output acceptance. The checker external-note cases still fail whole comparison despite retained note labels. All remaining outputs and comparator diffs are in remaining-failures.json.gz and captures.tar.gz.

79 invalid/nonpositive references remain separate:59 Rust failures and20 Rust successes, unchanged. The two outgoing unactivated returns are Java-invalid and earn no positive parity credit. Two Java-valid incoming short `?` arrows remain parser errors. The captured24 all have valid XML with SEQUENCE metadata, empty stderr and exit0; validity was independently checked by import_references.py, not inferred from exit0 alone.

All2,281 frozen full YAML stdout captures are byte-identical. Across the additional90 entries there are30 stdout changes:16 true/nondefault Display fields,10 retained Note events and4 parse recoveries. Old models without the fields deserialize and serialize identically in the focused round-trip test. Nineteen pre-existing YAML serialization panics remain in both revisions; raw stderr includes different process IDs and is preserved separately. AST and ASCII outputs were also captured for every entry. Ten ASCII stdout/status changes arise from parser ownership and valid terminator repair; this is not an ASCII behavior-invariance or ASCII parity claim. See model-ascii-yaml-summary.json and full final-model-and-ascii.json.gz.

Final broad gates ran once on stable code. Parser579, renderer798 (including12 ASCII), and CLI13 tests pass. Main library tiers remain product11181/11251 and assisted10497/11251 eligible (12550 total,1299 skipped), sequence1411/1411; these are library calls, not11251 CLI processes. Failure-name snapshots were copied immediately and match the prior packet without gains/losses. Main gates remain nonzero because baseline ratchets and assisted failures are unchanged. Standing perturbations:916/1576 matched,660 failures; corpus cwd is crates/rustuml-oracle and differs from the real CLI repo cwd. Guards4/6 remain red (decimal provenance and review records, including unreviewed new mechanisms). Owned formatting passes; workspace formatting still fails in class files. Clippy stops on the existing parser component dead function; renderer-only clippy reports the existing19 errors. No guards/baselines/comparator/allowlists were changed.

The2371 entries have2323 unique source byte hashes and48 duplicate entries; none removed. Fresh64, fresh24 and both handoffs are disjoint by source bytes from earlier entries. Parent1449 overlaps; it is not added to this denominator. See cohort-overlap.json for the historical parent overlap limitations.

Reference relay provenance is preserved raw: pinned JAR aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053, headless wrapper, GMT+11:00, PRIVATE_CWD substitution only, exact source/stdin/stream hashes, JSON-array argv. Returned gzip SHA09c8ae231e6842391d946c298b1bf41d3511d77d95d5e318cadaba109e376992. Java argv is derived from the wrapper, not process-traced. Remote OpenJDK21.0.5 and Graphviz12.2 differ from historical origin; scoped transport conformance is not global font/Graphviz equivalence. No local Java/javac or own SSH was used.

Reproduction: use all-inputs.json with the unchanged docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py and exact binaries.json bindings. Commands, environments, all raw reference captures, sources, initial failures, final SVGs, AST/ASCII/YAML outputs and logs are preserved in captures.tar.gz. final_checks.py lists exact broad commands; checks.json records exits. Private originals remain at /Users/marcelo/Library/Caches/rustuml-sequence-filtered-display-0cde. New dependencies are enumerated in dependencies.json; no target mutation or waiver was made. In particular short-arrow model/constraints, legacy external callback ordinates and external-note horizontal envelopes need separate source-derived follow-up; no coordinate compensation was attempted.
