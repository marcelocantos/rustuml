# Comment consumers and state fills — maker evidence, not acceptance

Production `523a2d89226d764b818b7af2472593d2690e9c4b`; before evidence `3875fee3b164d63d4b39df4bc6c13ca8ace89169` / production `346f6b3f680895d3eb457041ed88ae224c583d95`. Independent rejection `075f5095520bb1dc85a32c1f051a98421522419d` imported unchanged at `138ca3e5f`. All **11,553** historical review/account/perturbation files at that import remain byte-identical. No old review was rewritten. This packet requires a **new independent checker**; no mechanism is self-accepted.

**Account-format limitation:** the canonical-format `../account.json` index was created after production. The causal content predates production in the four original files at `42344228e` and `dda0437c0`, with exact-source correction at `27bb1c3ff`. The index only references those files and their commits. `missing-comments-account.json` and `follow-on-producers-account.json` retain their original object-shaped Rust-location arrays as a format deviation. No original bytes were rewritten or backdated. The new checker must judge chronology from commit ancestry, not this index. The affected review-record guard was rerun alone after indexing; it remains red because independent review is pending and historical violations remain.

The approved14-entry malformed list was committed at `0134c0f48`. Accounts precede production drafts at `42344228e`, the exact-source correction `27bb1c3ff`, and the three-site follow-on `dda0437c0`. The source correction explicitly replaces two nonexistent Java class names in the first state account; the original account is preserved. Java is pinned to `71806a23780b04a5ccde2f8ceb5121edad5eb711`, JAR `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

## Actual CLI measurements

These are source-only executable measurements using `-pipe` (default SVG) and `--yaml -pipe`, not library counts. The separate registered XML census uses `-pipe -tsvg`. Denominators retain every duplicate and nonpositive. The unchanged comparator/probe is hashed in binary-provenance.json.

|Cohort|Entries|Java-valid|Before passes|After passes|
|---|---:|---:|---:|---:|
|retained|1730|1609|709|709|
|maker18|18|17|13|13|
|sprites50|50|49|49|49|
|checker38|38|34|26|26|
|fresh15|15|15|13|13|
|fresh26|26|25|24|24|
|new15|15|14|12|12|
|delimiters4|4|4|0|0|
|focused2|2|2|2|2|
|checker30|30|26|22|22|
|supplement8|8|7|7|7|
|fresh12|12|12|11|11|
|checker28|28|28|23|23|
|fresh16|16|16|15|15|
|golden-regressions|6|6|6|6|
|checker36|36|34|19|19|
|fallback16|16|14|13|13|
|checker32|32|29|19|19|
|shared22|22|18|10|10|
|checker87|87|83|36|36|
|corrective16|16|15|11|11|
|checker54|54|46|16|20|
|consumer16|16|16|9|12|
|full-golden|12550|11251|11179|11179|

All retained cohorts including checker54: **1056→1060/2103 valid**,2261 entries. Including compact16: **1065→1072/2119 valid**,2277 entries. Checker54 preserves its eight nonpositive references and four parser panics. All16 compact references are independently classified valid from raw SVG and stderr; their exit0 status alone was not used as validity. Fresh gains are transparent/unknown named ordinary state fills and composite alpha; checker gains additionally include concurrent alpha. Every outcome, including whole failures, remains executable.

Across goldens and every cohort: **zero comparator lost passes, zero new process errors, zero new panics**. Goldens emit12470SVGs,80 existing process errors,0panics. Retained cohorts retain32panics; checker54 adds its four pre-existing panics. YAML covers14827 memberships:14819 byte-identical and **eight map-order-only differences**, zero semantic differences. No UID, identity, object, topology or model content changed; T14.6.6 order nondeterminism remains. The intermediate run independently had six order-only differences, not eight; both raw runs are preserved.

## Golden bytes and producer coverage

Exactly **194 golden SVG byte changes**, all comment-only. **188 now have exact Java producer-comment lists**; the other six retain existing producer/order/family disagreements. No XML-equivalent re-encoding occurred in this round, and no golden decoded attribute, painted text, element, coordinate or dimension changed.184 changed goldens remain passing;10 remain nonpassing/nonpositive;0new passes and0lost passes. golden-byte-summary.json lists every changed name in all categories.

The six remaining changed lists are:
- full-golden/combo/combo_embedded_state_machine
- full-golden/combo/combo_microservice_arch
- full-golden/combo/combo_state_everything
- full-golden/edge-cases/edge_misc_kitchen_sink_state
- full-golden/state/state_composite_entry_exit_concurrent
- full-golden/type-detection/ambiguous_allowmixing_sequence_component

Their full Java/before/after lists and validity are in producer-list-census.json in metadata. They are not waived. State consumer ordering/missing topology remains separate from comment escaping and local-name selection; wrong-family/error references remain nonpositive. Across the full corpus there are other unchanged pre-existing producer differences; the complete census retains them and this packet makes no global comment parity claim.

The seven valid name-selection cases from the rejection now match Java: six creole_in_stereotype_* controls and deploy_unicode. Hindi/Thai negative references remain nonpositive. Root class packages use raw Package.name; nested O.I versus Java I remains **T14.23**. Explicit class aliases use existing model identity projection. Deployment comments recover raw unaliased labels versus explicit IDs where distinguishable; ambiguous alias/derived-id cases remain T14.23. No source reparse or invented namespace separator was added. State's dot projection is backed by StateDiagram.java:62 and parse/state.rs:245–271, not inferred from fixture labels.

Eleven real legacy activity linear edge emitters now add Link.commentForSvg-derived comments. The approved follow-on adds Cluster.drawU comments at both existing state cluster wrappers, suppressing ## synthetic groups, and stops Class-kind comments for Object images because EntityImageObject emits none. `combo_class_vs_object` now has the exact Java producer list. The follow-on alone changes96 measured outputs (38goldens), all comment-only; all comparator verdicts are unchanged. No missing objects were synthesized.

The mechanical audit compares456 existing element macros:116activity,151class,83deployment,106state. After isolating intended comments and fill serialization, all templates and bound non-comment/non-fill expressions match the prior tree. The follow-on has its own byte audit. No sequence, parser/model, font, metrics, native/layout, TIM, registry, comparator, baseline, target or shipping change.

## State paint coverage and residuals

36 variable shape-fill sites route through existing shared fill_attributes; palette selection uses existing creole registry and shared normalize_color. Source flow is EntityImageStateCommon.applyColor→ValueImpl.asColor→HColorSet.getColorOrWhite; composite Cluster.drawUState and ConcurrentStates feed the same shape-driver serialization. Transparent survives as transparent paint rather than becoming white; CSS4-only RebeccaPurple falls back to pinned-Java white; RRGGBBAA reaches shared fill-opacity. Automatic background selection and gradients are explicitly deferred.

The post-change census leaves19 direct dynamic-looking fill sites:15fixed NOTE_FILL/PSEUDO_COLOR/BAR_COLOR constants and4TEXT_COLOR consumers (handwritten warning/history glyphs). Those four are text paint, not repaired state shape serialization. Choice/history style selection still differs even where color bytes serialize correctly. No geometry was adjusted to hide those failures.

Compact whole failures remain:
- raw_deployment_links: comments exact, but Java qualified-name metadata `.clair--`/`Peer.-` differs from raw Unicode Rust metadata. T14.23 identity projection.
- state_concurrent_alpha: paint repaired; Java Leaf UID ent0003 versus Rust ent0004. Allocation dependency, no model/UID rewrite.
- state_choice_alpha: Java default#F1F1F1 versus Rust global background#345678 opacity0.49804. Choice style selection remains wrong.
- state_history_names: comments exact, but Java108x235/21elements versus Rust193x223/20elements and wrong history marker shape/style. Geometry/consumer dependency.

The checker empty-state cases remain failures. Prior state_alpha_empty is Java50x40 versus Rust59.7578x50; the full per-case paint and geometry projections are retained. This is a separate handoff, not a paint workaround. Object safe_unicode_object/unicode_object parses with objects=[],links=[]; **T14.17** admission owns missing consumers. unicode_object is well-formed empty100x50SVG because the source label is never painted/unhidden; this is dropped-model divergence from Java's crash, not positive parity.

Four valid checker inputs remain parser panics at parse/class.rs:285:57, empty parts[0]: dash_01 (`class "--"`), dash_03 (`class "----"`), dash_06 (`class "-->"`), dash_07 (`class "<!--"`). SVG and YAML each exit101; exact sources/hashes/stderr are preserved. No parser fix.

## XML oracle and gates

The standing strict XML test passes17/17CLI tests, with26approved DOT doctypes. Final goldens:12550inputs,12470emitted,0malformed,80process errors,0panics. Registered perturbations:2139inputs,2024emitted,**14malformed**,115process errors including36panics. The before registered census was2123 before the16new pairs were registered; all16also have actual before CLI captures and are well-formed. xml-by-cohort.json provides each immutable cohort separately.

The exact14 malformed cases are the seven prior C0 entries plus the seven approved literal-E000 additions. Four additions are T14.20 Java crashSVGs; three are T14.21 syntax errors with T14.17 admission co-dependency. Each entry retains source hash, owner, reason and both ratchet directions. These14 outputs are **malformed Rust output**, never XML-passing. No stale entry, unregistered failure or loosened parser is hidden. The independently accepted DOCTYPE policy was not changed.

Final stable broad gates ran once after the three-site follow-on. The complete earlier gate run remains intermediate evidence. Parser624/624; renderer799/801 (same two failures: plain_theme_uses_legacy_class_icon_and_font_defaults and parser_source_identity_matches_java_shadow_id_after_initial_blanks); CLI17/17. Product library diagnostic11179/11251 and assisted10497/11251; both72/754 failure-name files are byte-identical to the exact baseline, no gained/lost library passes. Standing930/2139 remains red (1209failures). Guards4/6 remain red on decimal_literal_ratchet and parity_review_records_are_complete. Clippy remains red on parse/component.rs:72 dead code; fmt remains red on baseline class/parser formatting; diff check passes. No threshold or failing unit was edited.

Explicit retained dependencies include T14.12warning chrome, T14.13stripes, T14.14plain/code, T14.15automatic/background and font/alias/metric coupling, T14.17admission, T14.18gradients, T14.19base-style OR, T14.20crash contract, T14.21negative-family behavior, T14.23local identity. Sequence mixed-size baselines/note height, forced alignment, unknown/guillemet, Unicode/fonts/PLAIN, note and defined-sprite resolver ownership, URL/italic/legacy tspan, welcome and negative diagnostics remain failures. The salt/json process errors remain errors. Malformed scales/size zero/overflow and shape_direction errors remain nonpositive. Batik-limited math cases carry no positive math claim. No combined correctness claim with unintegrated sibling production.

## Provenance and reproduction

Private cache `/Users/marcelo/Library/Caches/rustuml-comment-state-consumers-maker`. `commands-and-statuses.json`, gates.json and every raw result.json retain commands/statuses; source/binary hashes are exposed here. Private targets use jobs4. Fresh compact16 used only the parent's existing Monkey Man relay, worker.C1mYwnev/new, raw GMT+11:00,env{}, noassets. ReturngzipSHA256 `3e5fc276a9a854fbb7a24a3720b5906e72e839e556202a2f90f3a5bd27b86219`; transportSHA256 `7e2fbcb62a5f38375cebaf6fb1a0765db7c2aac986e51a5830b3ae7700152339`. Exact sources/resources/membership/streams/pins/argv/cwd/TZ independently verified;0timeouts. Java argv is derived, not process-traced. Original envelope and lossless parent mapping are preserved. No local JVM/javac/SSH/new runner or child.

Archives retain every SVG/YAML/error/panic, immutable memberships and source/reference bytes, all intermediate captures/gates, source snapshots, scripts and initial setup failures. Input paths are original provenance; frozen-inputs-manifest.json maps them to content-addressed archived bytes. Relocate consistently for reproduction. Initial probe permission failure and the prematurely launched replay's executable-copy race are preserved; only complete stable-binary replays supply headline counts. Preliminary focused captures without a distinct saved binary hash are labeled non-headline. Archive entry manifests are SHA256-checked on readback. No global historical font/Graphviz equivalence is claimed.
