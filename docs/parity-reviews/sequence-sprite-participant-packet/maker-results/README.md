# Sprite ownership and participant execution: maker results

This packet requires a **new independent checker**. It does not accept or replace any earlier rejected claim. Final production is `ba2e6c2b79bb35841351ce2ed2e88e3a7abbe9d7`; baseline evidence is `c6295ccdef39719b60f0e88dfd9555919523c961`. Rejection `a2f0aa4083b8e1195f19e9434b6d6a4be7e407f9` was imported unchanged as `2b8dc61e28943773a71c13ffbbeb52a2d4500f8a`. All 14,413 previously tracked review/perturbation paths and all 1,267 imported paths remain byte-identical.

## Model changes and ownership

Only `crates/rustuml-parser/src/parse/sequence.rs` and the authorized portions of `crates/rustuml-parser/src/preprocess/mod.rs` changed. `preprocess-authorized.diff` records the exact enum/field/initializer and existing declaration/body-block changes. No other preprocessor/TIM code, dispatch, ASCII renderer, model schema, shared text/style/font code, targets, baselines or comparator changed.

* Sprite commands retain case/Pattern2 separator semantics. SVG and pixel bodies use their distinct Java END patterns; pixel END also accepts `endsprite` and `end[%s]?sprite`. Inline SVG does not open a multiline body. Pending source becomes blank placeholders only after completion. The pending span is a suffix count, so appending an included file's output cannot erase preceding commands. SVG/file/stdlib/compressed execution is still unsupported; consumption does not claim sprite rendering support.
* Participant declarations consume the whole supported grammar, preserving dot/@ identities, aliases, compact stereotypes/colours and Java option order. Negative ORDER retains supported declaration fields with an explicit unsupported-order warning. Signed/default-zero ordering and re-declaration ordering/englober effects remain dependencies.
* Single-line participant COLOR is validated as one HColorSet token; note/box colours retain Colors parsing. Direct gradients remain valid and warn, including `-`. Existing-participant early return avoids executing a new colour; multiline participant execution does not read its COLOR capture. Unsupported multiline participant rendering remains explicit.

All accounts predate their associated code. Exact mapping is in `implementation-map.json`:

| Mechanisms | Precommitted account | Implementation |
|---|---|---|
| Sprite block ownership; complete participant consumption; command-specific colour paths | `589b3d5e656a9a02b8e03f9f4fd103be084f2bef` | `d15d6eaf928d1b3eaf6aa49c9cf4d57b69e9caad` |
| Preserve supported declaration fields while reporting signed-order residue | `643f125a81867c6c492a59fd468346e31b18c3b1` | `d15d6eaf928d1b3eaf6aa49c9cf4d57b69e9caad` |
| Incomplete sprite admission | `0d9e70030847b1479275b26b8965c8735d172cfe` | `d15d6eaf928d1b3eaf6aa49c9cf4d57b69e9caad`, then suffix correction below |
| Pending sprite suffix across output buffers | `2268b0ddf9daf5752b885b14497dc80e7ea4e5fd` | `ba2e6c2b79bb35841351ce2ed2e88e3a7abbe9d7` |

`source-provenance.json` includes pinned Java file hashes and a new package-name clarification: FactorySequenceNoteCommand is in `command.note.sequence`, not `sequencediagram.command`. The original account is unchanged. HColorSet accepts direct gradients; the request's earlier composite-or-gradient wording was narrowed by source and reference evidence, not by narrowing valid grammar.

## Actual CLI measurements

All commands used exact original source bytes, the unchanged comparator, repo-root cwd, fixed debug date/timezone and the pinned stdlib path. Every source was captured as SVG, AST, ASCII and YAML. The private cache is `/Users/marcelo/Library/Caches/rustuml-sequence-sprite-participant-c629`. Binary/source bindings, complete commands, stdout/stderr/status and hashes are retained.

| Cohort | Before: match / mismatch / valid error / nonpositive | Final |
|---|---|---|
| Frozen 3,569 + independent 571 = 4,140 | 2,615 / 1,077 / 18 / 430 | 2,622 / 1,071 / 17 / 430 |
| Fresh maker 30 | 10 / 14 / 4 / 2 | 23 / 5 / 0 / 2 |
| Missing-terminator supplement 4 | 0 / 0 / 0 / 4 | 0 / 0 / 0 / 4 |
| All 4,174 memberships | 2,625 / 1,091 / 22 / 436 | 2,645 / 1,076 / 17 / 436 |

There are **20 gains, zero lost matches, zero lost rejections, zero new valid-input errors, zero CLI panics and zero timeouts**. Five old valid-input errors recover, including the introduced multiline SVG regression. Nine additional invalid inputs now reject. The 4,174 paths contain 4,122 distinct source hashes; no duplicates were removed. Six independent-checker standing aliases are not added again to the 571. `denominators.json` records overlap. These counts are not added to a growing parent integration cohort.

Explicit rejection contracts are separate from SVG matches:

* Historical nonpositive: 87/99 rejected before and after.
* Prior/fresh known Java-invalid: 272/302 → 281/302 rejected.
* 35 exporter-error references remain nonpositive; no attempt to reproduce the Java exporter crash.
* Both new participant-composite negatives and all four missing-terminator negatives reject. No negative receives positive SVG credit.

All **12,550 golden actual-CLI SVG bytes are unchanged**: 11,181 matches, 70 mismatches, 1,199 nonpositive references and 100 non-UTF-8 reference-decoding failures. Equal output bytes reuse the identical comparator result; changed outputs would invoke the comparator again. There are no golden family changes, lost passes or new errors.

Golden ASCII output is byte-identical. Eight golden sources have AST/YAML byte-order differences (five AST, six YAML, overlapping), caused by unchanged HashMap iteration in Dot attributes and DiagramMeta.sprites. Parsed YAML values agree for all eight. Two same-baseline-binary reruns produced 21 ordering differences; all raw streams and diffs remain in `golden-model-order-diagnostics.json.gz`. No serialization or comparator normalization was applied. Retained/new cohorts have 47 changed AST/YAML captures and 30 changed ASCII captures; exact changes, families, SVG text/element counts and participant/message counts are in the semantic/model inventories. No ASCII behavior-invariance claim is made.

## Gates and intermediate evidence

Final gates ran on `ba2e6c2b79bb35841351ce2ed2e88e3a7abbe9d7`, with private Cargo target and jobs=4:

* Parser units **594/594**, renderer units **801/801** (including ASCII), CLI pipe tests **13/13**.
* Source-only **library** tier **11,181/11,251**, assisted **library** tier **10,497/11,251**; sequence **1,411/1,411**. Both commands still exit 101 under unchanged historical ratchets. Failure-name files were copied immediately and are byte-identical to the prior packet.
* Standing library perturbations: **3,259 entries, 1,434 failures**. This includes invalid references and six explicit aliases, and differs from the actual-CLI retained corpus. Every failure is listed. No exclusions changed.
* Guards **4/6**; provenance/attestation and decimal-literal gates remain red. No self-authored review records were added.
* Owned-file formatting passes. Workspace formatting remains red in unmodified class code. Clippy remains red on the existing unused component helper; renderer-only no-deps clippy reports 19 existing diagnostics in unchanged code. Exact commands/nonzero outputs are retained.

The first candidate lost `participant_negative_legacy` by dropping its entire declaration. Its source, before/after outputs, patch and binary binding remain intact; account 643f125a8 records the correction. Candidate2 restores that match. The first broad run on d15d6eaf9 remains under `first-stable-*`. A newly added focused test then proved an absolute pending-output index erased preceding declarations when a sprite opened in an include. Its failing and passing logs remain. Account 2268b0ddf preceded the suffix-count fix; that production change justified repeating the full measurement and broad gates. No prior run was overwritten or relabelled as final.

## Reference provenance and remaining failures

The 30-case and four-case batches were captured only through the parent's Monkey Man relay, with original manifest/bundle bytes, JSON-array argv, PRIVATE_CWD substitution and GMT+11:00. Pinned JAR SHA256 is `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`. Remote OpenJDK 21.0.5, Graphviz 12.2.0, headless AWT, en_AU and UTF-8 are recorded per batch. Java argv is wrapper-derived, not process-traced. No local Java/javac or SSH ran. Remote agreement is scoped, not proof of global equivalence with the historical origin environment.

Independent raw XML/error classification finds 28 valid SVGs and six Java command errors, with no new intended/observed disagreement. SVG unclosed controls show Syntax Error; pixel unclosed controls fall through to CommandSpriteFile and show Cannot read: [2x2/16] {. Rust rejects both, but the diagnostic text differs; no diagnostic-equivalence claim is made. Source/reference pairs, transport/stream hashes, parent validations and every failed control are preserved unchanged. The original checker disagreements remain untouched.

`remaining-failures.json` lists all **1,076 valid mismatches, 17 valid CLI errors and 436 nonpositives**. The valid errors are the unchanged 15 parent-approved T14.21/T14.22 paths plus two pre-existing short-external-CREATE errors. No new exception is claimed. Fresh residuals are the two participant gradients, two note/box colour cases and legacy multiline participant display. All raw outputs remain active evidence.

`dependencies.json` records unsupported sprite execution, signed order/default-zero insertion and re-declaration englober/order behavior, multiline participant execution, full gradient/style painting, reader/link/factory problems, T14.13 headings, T14.11 fonts, T14.20 exporter references, note/group/box/page/envelope geometry and frozen failures. None is waived.

## Replay material

`all-inputs.json` is the durable 4,174-member source/reference manifest. `before.json.gz` and `after.json.gz` preserve actual outcomes; `capture-path-mapping.json` maps new private inputs to byte-identical durable copies. `captures.tar.gz` contains every original capture, model/YAML/ASCII output, intermediate failure and script except executable binaries (their exact hashes are bound). `archive.json` and `capture-inventory.json` permit checksum validation. Older `final-*` command strings refer to the first stable run; `first-stable-path-mapping.json` records the lossless rename.

Typical local replay (no Java): set the replay script's cache path, provide a freshly built `final-bin/rustuml` and the unchanged `before-bin/compare_cli`, then run `uv run --no-project python replay.py final all-inputs.json` from the repository root. The committed scripts preserve the actual original invocation convention; do not overwrite an existing capture directory. Required gates and environment are recorded in `final_checks.py` and `final-checks.json`.
