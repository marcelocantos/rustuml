The class/container recovery packet is **rejected**. This independent checker accepts only `class-symbol-container-alias-identity` and `class-cluster-header-shape-extent`, at the complete tested revision. The other 22 new account records are rejected. These narrow acceptances do not certify an extracted patch, the old factory-precedence claim, or the packet as a whole.

The checker ran as `t3-2740aefa-independent-class-container-checker`, using inherited `gpt-6-astra` (high) through Codex as requested, with no children. Production was not edited. Original accounts, maker records, historical reviews, old fixtures, comparator, baselines, skips, error census and allow-lists remain unchanged. No target, parent checkout, remote branch, PR or release was changed.

**Exact preparation and chronology.** The prepared checkout started at `ea136c78ff71d2be074b39d2ffe0484decba0aec`. All 21 commits, in the exact order from `git log --reverse 8a61fa097d278dad508da0659953a98765926774..ad0690974a581caf7770178b5dcca0b01f72c086`, were cherry-picked. The combined tested production revision is `fefca37686ff0c93b5b306831e4554a2d8f9efa6`. The original implementation `18916d312c32340b0a674e15259dc3031ea57acf` became `2a8852f3d7cc2b9103efc14e8fe2ddfdf30318c6`; continuation `bed3b50ca08e12d8ada612791cf07393617b2041` became `58a89ed7f3fecece53659b097cd6a9af2952147e`. Both complete implementation diffs were inspected and are retained here.

`chronology.json` maps every original/picked hash. `account-map.json` independently maps all 24 accounts, their amendments/schema projections, and their actual implementation phase. Every original causal text predates its relevant production change. The four initial dependency plans explicitly deferred implementation; their concrete continuation accounts precede `bed3b50c`. The `aa5bb750` per-owner-origin plan was superseded by `c32b7c09`'s one-global-origin plan before that implementation. This is legitimate durable chronology, not a post-hoc rewrite. The later schema projections preserve the earlier causal text; they are not the first causal support. No missing preimplementation account was found for the mapped production mechanisms. The substantive problem is incomplete or incorrect realization of those accounts, detailed below.

Both original production phases were assessed across parser `diagram/class.rs`, `parse/class.rs`, `parse/mod.rs`, the component bracket-helper removal, renderer `class.rs`, the existing compound-clipping visibility change in `component.rs`, and the mechanical `ascii_class.rs` constructor updates. The full packet is 2,064 added / 438 removed production lines across seven files. The helper exposure itself does not introduce a new clipping algorithm.

**Reference and execution contract.** Java source revision is `71806a23780b04a5ccde2f8ceb5121edad5eb711`, read-only at `/Users/marcelo/work/github.com/plantuml/plantuml`. New references were generated with the absolute Temurin 21 binary and the source-pinned `plantuml-1.2026.3beta6.jar`, SHA-256 `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`, using `-Djava.awt.headless=true -tsvg -pipe`. These are raw CLI-pipe references, not historical GET responses. The JAR is not claimed historically byte-identical; font and external/vendored Graphviz equivalence are not assumed. Raw Java DOT and solver outputs explicitly test the latter for three cases only.

Golden submodule `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a` was initialized as a real submodule clone, using `git -c protocol.file.allow=always submodule update --init test-diagrams/golden` after the ordinary local transport was refused. No symlink was used. Stdlib was read-only at `/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib`.

During this review the owner approved raw Java CLI/file semantics, recorded in parent policy commit `b62d8ce967b6dda7d85bb6dc28ff92509f6066f9`. T14.6.3 is no longer pending. This checker read that exact policy from git without importing it or the TIM production/assertion changes. Independent raw-reader verification belongs to the TIM checker and was not duplicated here. New sources avoid outer body blanks; an internal header blank remains a real tested input and was not stripped. Existing references and sources were copied byte-for-byte. T14.6.2 cwd dependence and T14.10 file-base behavior remain visible; neither fixtures nor assertions were adjusted to hide them. Root-cwd isolated CLI theme results differ from the maker's separate/Cargo-cwd cohort. This is reported rather than normalized into a geometry claim.

The checker used `/tmp/rustuml-checker2740-target`, `CARGO_BUILD_JOBS=4`, and a fresh release workspace build. CLI comparisons use the actual `rustuml -tsvg FILE` product path. `--ast` is separately recorded diagnostic information only. The comparator helper calls the unchanged `rustuml_oracle::compare::compare_svg_strict`; it has the original invalid-Java-reference classification. No loose comparator or pixel tolerance was introduced. CLI exit status is recorded independently from SVG status. Invalid Java references are negatives, never passing comparisons.

**Measured results.** The current-parent inventory was frozen at 955 source/reference pairs, replayed through independently built parent and candidate binaries, with a separate process per input. There are zero pass-to-nonpass regressions in that inventory, but there are five new valid-input parse errors. A previously wrong image becoming an error is a regression even though a pass-count-only comparison misses it.

| Cohort | Parent pass / mismatch / process error / invalid reference | Candidate pass / mismatch / process error / invalid reference |
|---|---:|---:|
| Original 810 | 349 / 454 / 0 / 7 | 526 / 277 / 0 / 7 |
| Earlier class checker 23 | 0 / 23 / 0 / 0 | 23 / 0 / 0 / 0 |
| Sequence 41 | 17 / 22 / 2 / 0 | 17 / 22 / 2 / 0 |
| Theme 33 | 6 / 26 / 0 / 1 | 8 / 22 / 2 / 1 |
| Parent registry 6 | 6 / 0 / 0 / 0 | 6 / 0 / 0 / 0 |
| TIM 42 | 1 / 40 / 0 / 1 | 3 / 35 / 3 / 1 |
| **Current 955** | **379 / 565 / 2 / 9** | **583 / 356 / 7 / 9** |

The candidate's seven process errors are five parse failures plus the same two sequence panics at `sequence.rs:8022`. They do not cascade into later inputs. Nine invalid references are the eight preexisting original/theme references plus one TIM invalid control. The five new valid parse errors are:

- `tim-evaluated-event-stream/checker_comments_and_origins.puml`: trailing inline comment survives to the strict class parser.
- `tim-procedure-global-fork-memory/checker_explicit_override_survives_global.puml`: residual `!global argument` command.
- `tim-procedure-global-fork-memory/checker_global_overtakes_auto_local.puml`: residual `!global automatic` command.
- `tim-theme-post-expansion-source-identity/checker_dump_memory.puml`: generated dump comment reaches the parser as an unknown command.
- `tim-theme-post-expansion-source-identity/checker_inner_comment.puml`: trailing `/'` comment reaches declaration parsing.

All 31 prior maker cases pass independently. Of the 31 fresh maker cases, 30 pass and `group_lr` fails. All 62 are retained as executable pairs, with their original Java bytes. The initial checker matrix had 135 pairs: 80 pass, 46 mismatch, five parse errors, four invalid Java references. Four final source-guided attempts against narrowly passing identity mechanisms added two passes and two mismatches. Thus the **139 checker pairs yield 82 pass, 48 mismatch, five parse errors, four invalid references**. Eight of these pairs deliberately repeat successor probes under their original dependency accounts; they are not 139 unique behavioral inventions.

The four invalid fresh references are the explicit late-mixing and malformed `*class` negatives plus two unexpected link-before-note cases rejected by Java because an existing leaf cannot be rematerialized as a note. All generated source and Java error SVG bytes remain present. The note cases did not become positive controls.

There are 237 added executable entries: 139 checker-generated pairs, 62 retained maker pairs and 36 explicitly labeled dependency/current-failure copies. These copies are for per-mechanism enforcement, not additional independent gains. The final actual CLI replay of **all 1,192 entries** produces **726 pass, 429 SVG mismatch, 24 process errors, 13 invalid references**. The 24 errors include the same two sequence panics and 22 parse errors, counting deliberate duplicate enforcement entries. `final-results.json` contains every outcome; `retained-manifest.json` distinguishes origins and hashes.

| Gate | Observed result |
|---|---|
| Release workspace library tests | 1,428 pass: parser 571, renderer 775, layout 41, math 10, oracle 31 |
| CLI pipe integration tests | 13/13 pass |
| Workspace check / release build / format check | Pass |
| Source-only main tier | 11,188 / 11,251 eligible; 63 mismatches; ratchet test remains red |
| Oracle-assisted main tier | 10,497 / 11,251 eligible; 754 mismatches; test remains red |
| Full golden inventory | 12,550; 1,299 skipped by unchanged existing policy |
| Documentation assertions | 2/2 pass |
| Harness guards | 4/6 pass; decimal ratchet and review evidence guard remain red |
| Built-in full perturbation gate | Red, with invalid references, retained mismatches, parse errors and two sequence panics visible |
| Clippy with denied warnings | Red; 21 diagnostics, no cleanup performed |

The source-only result is seven above the stated parent/root result. Its frozen checked-in ratchet has additional earlier unlocked gains, so the ratchet error lists more than seven; neither baseline was rewritten. The assisted total is unchanged from the stated root/maker measurement. The current955 parent replay is a direct independent before/after measurement; the two main-tier parent figures are not represented as a second parent run by this checker. Rejected new reviews deliberately leave the unchanged review gate red. These are evidence of rejection, not a request to weaken the gate.

**Per-mechanism decisions.** Counts below are the fresh probes assigned directly to each account, before retained dependency copies. The final identity probes are included. Every account also has its own schema-v1 `review.json` with commands, full original/picked hashes, findings, dependencies and all its executable pairs. No omnibus attestation substitutes for these records.

| Mechanism (all prefixed `class-`) | Verdict | Direct fresh result and principal reason |
|---|---|---|
| command-execution-ownership | Rejected | 5 pass, 5 mismatch, 1 parse error, 1 negative; root native kinds bypass the successful class trial; braced notes lack ownership |
| symbol-container-alias-identity | **Accepted narrowly** | 8/8 pass; forward/reverse/reopened/dotted aliases, opaque displays, nesting and LR |
| native-leaf-presentation | Rejected | 3 pass, 6 mismatch, 1 negative; bodyless declaration visibility passes, member-bearing/icon-size axis fails |
| note-target-quark | Rejected | 4 pass, 2 mismatch; group targets are excluded from entity-only lookup and fallback loses a class |
| quoted-code-identity | Rejected | 4 pass, 2 mismatch; identity survives but implicit-namespace note emission and LR note placement fail |
| cloud-child-coordinate-dependency | Rejected | 2 simple passes; concrete successor's nested/root-note failures block closure |
| description-leaf-image-dependency | Rejected | 2 mismatches; implemented successor remains incomplete |
| note-cluster-ownership-dependency | Rejected | 1 pass, 1 mismatch; external-note/envelope dependency remains |
| page-chrome-composition-dependency | Rejected | 2 default passes; successor's current-style matrix fails |
| command-grammar-closure | Rejected | 2 pass, 4 mismatch; mixed kinds retained but image path incomplete, further grammar failures retained |
| current-spot-cascade | Rejected | 4 pass, 8 mismatch; narrow palette flow corroborated, full native/custom/group prerequisites fail |
| description-symbol-images | Rejected | 3 pass, 6 mismatch; UML1, inner label alignment, boundary ordering and missing leaf images |
| chrome-bordered-block-geometry | Rejected | 6 pass, 2 mismatch; style-selected font/alignment not propagated |
| note-declaration-scope | Rejected | 3 pass, 1 mismatch; root-note frontier still wrong |
| svek-painted-origin | Rejected | 4 simple passes; retained groups/root notes violate the complete painted origin |
| group-paint-ownership | Rejected | 4 URL passes; final group translation and current-theme paint dependencies fail |
| parenthesis-link-identity | Rejected | 4 direct passes; relevant group endpoint dependencies fail |
| package-command-precedence-closure | Rejected | 4 pass, 2 parse errors; ordinary package visibility grammar omitted |
| group-endpoint-routing | Rejected | 5 pass, 3 mismatch; fresh LR group and two shielded-interface directions fail |
| container-only-factory-closure | Rejected | 2 pass, 2 mismatch; empty node/cloud path retains AST but misrenders |
| named-note-scoped-identity-closure | Rejected | 2 pass, 2 parse errors, 2 negatives; qualified note codes rejected |
| group-magnetic-extremities | Rejected | 4 direct passes; group-image coordinate dependency fails |
| cluster-header-shape-extent | **Accepted narrowly** | 4/4 pass; shape supplement correctly follows title/stereotype maximum |
| single-svek-envelope-origin | Rejected | 2 pass, 4 mismatch; final x translation still splits group and leaf/edge owners |

The two accepted invariants were separately inspected against Java dataflow and tested with fresh inputs, rather than inferred from main-corpus gains. Alias identity is independent of whether an unrelated command factory accepts a stream. Cluster header extent is a local ordering of an existing source-derived supplement. Neither is an independently tested git commit: the implementations share parser/renderer hunks with rejected mechanisms. The parent must not extract either from `18916d31`/`bed3b50c` and assume the new combination has this attestation. Preserve the full revision dependency or independently build and recheck a proposed extraction. There is no palette-only or factory-only waiver here, and the rejected historical `class-native-container-factory-precedence` review is untouched.

**Source/model findings and constant audit.** The actual Java methods, not just account titles, were inspected. Full implementation diffs and source-file hashes are retained. The parser now truly shares `parse_commands` between viability and execution, but `native_container_candidate` still requires a package, and `detect_uml_subtype` still has a scoring fallback. Root `record`/`struct` therefore choose Sequence despite a consumable class stream. `ClassDiagramFactory` registers the braced form of `CommandFactoryNoteOnEntity`; Rust does not. `CommandPackage` supports `[+~#-]`; Rust's ordinary grammar omits it. `CommandFactoryNote` permits dotted codes; Rust's floating-note alias regex is word-only. `CommandFactoryNoteOnEntity` resolves any quark data, whereas Rust looks only in its entity map and excludes groups. These are named command/model divergences, not recommendations to add fixture-shaped special cases.

The mixed-leaf renderer selects a `DescriptionLeafStyle` only for component/boundary. Other accepted description kinds remain class boxes. `EntityImageDescription` chooses its USymbol and creates text with the style/BodyFactory inner alignment; Rust always chooses Component2 for a component and centers each line. In `plain_multiline_component`, the short label's Java x is 22 while Rust centers it at 70.8906. The equivalent boundary short label differs by the same 48.8906. `USymbolSimpleAbstract` also explains the boundary label/stereotype drawing order difference. These are distinct from the verified outer dimensions.

Numerical provenance was checked in the actual sources:

- Component margins `(15,25,20,10)`, three small symbol rectangles, and boundary radius12/arm17/margin4 imply the implemented 40/30 and 49/32 dimensions. The numbers are supported, but correct numbers do not fix style selection or multiline alignment.
- Stereotype width `+2` is `EntityImageDescription`'s one-pixel margin on each side. Reusing the same font/style for all description text is still incomplete.
- Action/process/queue margins and polygon offsets10/queue dx5 match `USymbolAction`, `USymbolProcess`, `USymbolQueue`; empty action/process title height10 and stereotype x offset4 match their `asBig` path.
- Native spot characters/colors and declaration visibility icon geometry correspond to `EntityImageClassHeader`, `VisibilityModifier` and `plantuml.skin`. Body/member icon geometry remains an explicit failing prerequisite. Current spot colors trace to the current style builder; stale body/group style and custom-spot dimensions still fail product comparisons.
- Border extent `+1` is supported by `TextBlockBordered.calculateDimension`. That does not justify defaulting away header style FontSize18 or alignment in the failing probes.
- Polygon overscan10 and rectangle inset1 are `LimitFinder` behavior. Cloud frontier measurement and two-decimal SVG-origin serialization have source counterparts. The changed `note_dx/dy` branch retains an older `-1` but compares raw cluster/node minima; merely citing LimitFinder does not derive that branch. No complete shape/frontier derivation was found, and root-note counterexamples remain.
- Parenthesis radius9, half-angle70 degrees, decoration length10 and stroke1.5 exactly match `ExtremityParenthesis`. Folder title margins3+3 and shoulder slopes7/14 match `USymbolFolder` magnetic-border construction. These do not establish a correct final coordinate frame.
- The existing shared shield table's rounded width and floored vertical margins are not equivalent to Java's three-row fractional HTML table. Its unchanged presence is a blocking dependency, not new candidate constant provenance.

No new production golden reads, fixture-label conditionals, per-branch-count constant ladders or post-hoc SVG string surgery were found in the reviewed range. Numeric route serialization occurs in the model before drawing, not via an SVG text replacement. Existing assisted replay/grid fallbacks were not introduced by this packet and are not evidence for source-only acceptance. The global decimal ratchet remains red (including `class.rs` 383 allowed versus578 observed); the independent lexical census is exactly578 at the candidate start, after the first implementation, after the continuation and in the combined tree. This packet adds no high-precision literal census growth. The frozen ratchet remains unchanged.

**Repair packet for the three named layout residuals.** The evidence narrows the maker's proposed shared-layout explanation. Two cases have a class-renderer origin bug; one has a shared shield-table model limitation. There is no reason to tune Graphviz constants for the two group cases on the evidence collected.

1. `package_target_parenthesis_one_dash_lr`: Java and Rust leaf rectangles coincide at `(85.96,68)` and `(85.38,164)`; the visible spline agrees to serialization tolerance. Java's group begins at x61.96, Rust's at x6: the group omits the same 55.96 left-frontier translation used by its leaves and edge. Canvas width is280 versus257. `layout_x_bias` is computed in `render_plantuml_svg`; entities/edges add it, but `layout_package_clusters` receives no bias and records `x=pos.x+MARGIN`. Repair the single final-origin application to nonempty groups and canvas bounds. Preserve serialization stages rather than adding55.96 as a constant. The exact Java DOT produces matching cluster/node coordinates through the vendored solver.

2. Fresh maker `group_lr`: Java group D begins at x71.105, Rust at6; A is75.905 versus10.7954 (the small additional difference is serialization). Both leaf rectangles and the visible link path/decorations are already identical. Java canvas width225 versus Rust201 follows the same omitted group translation. Fresh checker `groups_3` independently exercises the failure with different labels and widths. Apply the same shared final-origin repair across all painted owners, then test both endpoint orders, LR/TB, wide/short labels, one/two groups, root notes, nested shapes and decorations. Do not alter shared routing to compensate. The raw Java DOT solve is again equivalent through both engines.

3. `followup3_upper_interface_bare_lr`: Java `SvekNode.appendHtml` emits a three-by-three shield with a central18x18 cell/port `h`, left/right fractional41.33984375 cells and vertical16.48828125 cells for this input. Running that exact DOT through the vendored build reports `Not built with libexpat. Table formatting is not available.` and `port h unrecognized`. The Java/external width is1.6111 inches while the unsupported native path is0.7840033637152778 inches. Existing `graph.rs::svek_shielded_node_table` substitutes one rounded cell and a compensating port, which does not model the table. Repair must supply faithful static table support or a proven equivalent cell/port layout model retaining fractional shields and central-port semantics. The binary must remain standalone. Test both directions, varied label widths/stereotypes, declaration orders and links to both ends. Fresh `shield_TB` and `shield_LR` are blocking counterexamples as well as the original.

`java-trace/{package_target,group_lr,shield}` retains raw source, Java image, SVEK DOT/SVG, external DOT output and vendored diagnostic output. `solver-commands.json` records exact diagnostic calls. `scripts/checker2740_raw_dot.rs` only diagnoses the pinned native library; it changes no production layout. The solver observation is local to these inputs and does not prove global Graphviz-version equivalence.

Additional repairs must cover command ownership/fallback without erasing native objects; missing ordinary package and named-note grammar; actual symbol image/style and inner text alignment; chrome style flow; note membership versus final image frontier; and implicit namespace emission/LR note direction. Keep each repair's causal account and independent held-outs. Existing global failures and invalid controls are not waived because file ownership or implementation time is limited.

**Evidence and replay.** `current-results.json`, `parent-current-results.json`, `cohort-comparison.json`, `maker-results.json`, `fresh-results.json`, `acceptance-results.json` and `final-results.json` retain independent statuses, exact argv, binary/comparator hashes, input/reference hashes and per-input outputs. Candidate, parent and final output archives include every CLI SVG, AST diagnostic, stderr and strict diff. Raw Java generation metadata is in the three `generation*.json` files. `artifact-hashes.json` hashes the final support artifacts; `inspection-sources.json` pins inspected source files. `logs/` contains unit, main-tier, guard, format, clippy, CLI and final-gate results. Scripts record the actual scratch paths used and are included for audit, not installed as a new harness.

The durable enforcement point is the existing unchanged `parity_perturbations` test, which discovers all new pairs. Run it and `harness_guards` after any repair; both are intentionally red now. The checker has committed evidence rather than production fixes or acceptance waivers.

The staged `git diff --cached --check` exits2 on preserved raw diff/log whitespace. Those evidence bytes were not normalized. The production format check passed, and the staged ownership audit found only new checker records/support and new perturbation pairs.
