# Shared SVG correction — maker evidence, not acceptance

Production `346f6b3f680895d3eb457041ed88ae224c583d95`. Stop for a NEW independent checker; no maker-authored review or attestation. Exact starting evidence `a2167f93695497e49b8ae71c4830bac96309023b`, production `a33dccc8e94dc44b5bafdc756fc1b88c42823a7d`. Rejection `4931481fd495ffff9da55a0c7514f43a487b087e` was imported unchanged as `2ed88eb55ec5f0974ec2830deccfcc401f6d9885`. All **11,325** prior review/account/perturbation files, including that rejection, are byte-identical. No sequence, parser/model, font/metric, layout/native, TIM, comparator, baseline, target, skip or allowlist production edits. No push, PR or release.

## Precommitted causal scope

- `741d1785a`: exact four approved C0 addendum entries plus proposal. Seven total; these remain malformed Rust outputs owned by T14.20, not XML passes.
- `4157ec70f`: DOCTYPE oracle account, before test draft. `47dece176` implements prolog-position eligibility, exact standard SVG1.1 PUBLIC bytes and internal-subset rejection. The exact checker comment-decoy is a standing negative test.
- `db8955411`: comment account, exact45file/line/macros and state-fill account before production drafts. Nine production files are named there. The mechanical audit verifies all45outside-comment templates and all non-comment arguments unchanged. Five macros also contain SVG elements; those attributes and geometry are unchanged. No comment was removed.
- `346f6b3f6`: implementation and all16fresh pairs. Shared whole-comment writer separates adjacent dashes, pads a terminal dash and reproduces Java's US-ASCII scalar replacement. Two ordinary state rectangle fills use the existing shared RGB/alpha serializer; other state shape paths were not expanded.

Java trace: EntityImageClass/EntityImageDescription/Cluster/SvekEdge→Link producers → UGraphicSvg.drawComment → SvgGraphics DOMComment → OpenJDK21.0.5 ToStream.comment:2312–2333. SvgGraphics.getTransformer uses lowercase us-ascii; Encodings.getWriter/StreamEncoder supplies replacement `?` for unmappable scalars. Literal ASCII &, angle brackets and quotes are retained inside comments. The state fill trace follows EntityImageStateEmptyDescription/EntityImageState → DriverRectangleSvg → shared fillMe serialization. Exact source files/hashes, accounts and raw captures are retained.

## Actual CLI comparisons

Replay commands use source-only `-pipe` (default SVG), separate `--yaml -pipe`, the unchanged comparator, private before/after binaries and jobs 4. The registered strict-XML census explicitly uses `-pipe -tsvg`. Exact per-call argv/cwd/environment/statuses are in the captures. Denominators retain duplicates and every nonpositive. These are CLI observations, not library counts.

| Cohort | Entries | Valid references | Before passes | After passes |
|---|---:|---:|---:|---:|
| retained | 1730 | 1609 | 709 | 709 |
| maker18 | 18 | 17 | 13 | 13 |
| sprites50 | 50 | 49 | 49 | 49 |
| checker38 | 38 | 34 | 26 | 26 |
| fresh15 | 15 | 15 | 13 | 13 |
| fresh26 | 26 | 25 | 24 | 24 |
| new15 | 15 | 14 | 12 | 12 |
| delimiters4 | 4 | 4 | 0 | 0 |
| focused2 | 2 | 2 | 2 | 2 |
| checker30 | 30 | 26 | 22 | 22 |
| supplement8 | 8 | 7 | 7 | 7 |
| fresh12 | 12 | 12 | 11 | 11 |
| checker28 | 28 | 28 | 23 | 23 |
| fresh16 | 16 | 16 | 15 | 15 |
| golden-regressions | 6 | 6 | 6 | 6 |
| checker36 | 36 | 34 | 19 | 19 |
| fallback16 | 16 | 14 | 13 | 13 |
| checker32 | 32 | 29 | 19 | 19 |
| shared22 | 22 | 18 | 10 | 10 |
| checker87 | 87 | 83 | 34 | 36 |
| corrective16 | 16 | 15 | 1 | 11 |
| full-golden | 12550 | 11251 | 11179 | 11179 |

The 20 retained cohorts total **2,191 entries / 2,042 valid**, **1,027→1,029**. Including compact16 gives **2,207 / 2,057 valid**, **1,028→1,040**. Zero lost comparator passes, zero new errors and zero new panics across every membership and full golden corpus. SVG and YAML command failures/panics are counted separately in command-outcomes.json. The six monospace regression controls remain6/6. Existing32sequence panics and salt/json process errors remain failures.

Full golden: **11,179→11,179/11,251 eligible**, all 12,550 sources measured;12,470 emit SVG and 80 existing process errors. **39 byte changes**, all semantic comment-body serialization, **zero XML-equivalent re-encodings in this correction**. Of these37 stay passing and2 (Hindi/Thai Java-negative references) stay nonpositive; no newly passing or failing golden. Every filename, complete decoded delta and source/reference path is in golden-byte-summary.json and changed-outputs-classified.json. Attributes, painted text and geometry are unchanged in all 39. Thirty changed producer-comment lists now exactly match Java; seven retain existing label/alias selection differences and two have Java-negative references without comments. These semantic differences were investigated; none are disguised as equivalent encodings.

YAML: 14,757 captured entries including duplicates (12,550 golden + 2,207 cohort).14,750 byte-identical; one sprite-map and six DOT attribute-map order changes. **Zero semantic model, identity, UID, object or content changes**. Full before/after YAML and parsed tagged comparisons remain under model-observations.json/archives. Map order is T14.6.6.

## Strict XML and oracle fault injection

The standing CLI test parses every emitted golden and registered perturbation SVG with roxmltree0.20. All other parser options and internal entity limits stay default. No external DTD/entity is fetched or resolved. Only the exact declaration at the eligible prolog position enables DTD parsing; a comment or PI decoy does not. Twenty-six actual DOT outputs pass. Before guard: real control and decoy both exit0. After guard: real control exit0, decoy exit101. Both malformed-ratchet directions remain unchanged, with exact source hashes and owner/reason data.

Registered perturbations: before 2,053 pairs emits 1,942 SVGs, 8 malformed; fresh 16 before emits 16 SVGs, 8 malformed. Combined same 2,069 inputs therefore have 16 malformed before. After 2,069 pairs emits 1,958 SVGs, **7malformed**,111 process errors including 32 panics. Only the exact seven approved C0cases remain malformed; all nine measured malformed comments are repaired. No expected-failure expansion beyond the approved seven. Strict standing gate passes 17/17; passing the ratchet does not relabel those seven outputs as wellformed.

| Corpus | Phase | Inputs | Emitted | Malformed | Process errors | Panics |
|---|---|---:|---:|---:|---:|---:|
| checker28 | before | 28 | 28 | 0 | 0 | 0 |
| checker28 | after | 28 | 28 | 0 | 0 | 0 |
| checker30 | before | 30 | 30 | 0 | 0 | 0 |
| checker30 | after | 30 | 30 | 0 | 0 | 0 |
| checker32 | before | 32 | 32 | 0 | 0 | 0 |
| checker32 | after | 32 | 32 | 0 | 0 | 0 |
| checker36 | before | 36 | 36 | 0 | 0 | 0 |
| checker36 | after | 36 | 36 | 0 | 0 | 0 |
| checker38 | before | 38 | 38 | 0 | 0 | 0 |
| checker38 | after | 38 | 38 | 0 | 0 | 0 |
| checker87 | before | 87 | 85 | 5 | 2 | 0 |
| checker87 | after | 87 | 85 | 4 | 2 | 0 |
| corrective16 | before | 16 | 16 | 8 | 0 | 0 |
| corrective16 | after | 16 | 16 | 0 | 0 | 0 |
| delimiters4 | before | 4 | 4 | 0 | 0 | 0 |
| delimiters4 | after | 4 | 4 | 0 | 0 | 0 |
| fallback16 | before | 16 | 16 | 0 | 0 | 0 |
| fallback16 | after | 16 | 16 | 0 | 0 | 0 |
| focused2 | before | 2 | 2 | 0 | 0 | 0 |
| focused2 | after | 2 | 2 | 0 | 0 | 0 |
| fresh12 | before | 12 | 12 | 0 | 0 | 0 |
| fresh12 | after | 12 | 12 | 0 | 0 | 0 |
| fresh15 | before | 15 | 15 | 0 | 0 | 0 |
| fresh15 | after | 15 | 15 | 0 | 0 | 0 |
| fresh16 | before | 16 | 16 | 0 | 0 | 0 |
| fresh16 | after | 16 | 16 | 0 | 0 | 0 |
| fresh26 | before | 26 | 26 | 0 | 0 | 0 |
| fresh26 | after | 26 | 26 | 0 | 0 | 0 |
| full-golden | before | 12550 | 12470 | 0 | 80 | 0 |
| full-golden | after | 12550 | 12470 | 0 | 80 | 0 |
| golden-regressions | before | 6 | 6 | 0 | 0 | 0 |
| golden-regressions | after | 6 | 6 | 0 | 0 | 0 |
| maker18 | before | 18 | 18 | 0 | 0 | 0 |
| maker18 | after | 18 | 18 | 0 | 0 | 0 |
| new15 | before | 15 | 15 | 0 | 0 | 0 |
| new15 | after | 15 | 15 | 0 | 0 | 0 |
| retained | before | 1730 | 1605 | 0 | 125 | 32 |
| retained | after | 1730 | 1605 | 0 | 125 | 32 |
| shared22 | before | 22 | 22 | 3 | 0 | 0 |
| shared22 | after | 22 | 22 | 3 | 0 | 0 |
| sprites50 | before | 50 | 50 | 0 | 0 | 0 |
| sprites50 | after | 50 | 50 | 0 | 0 | 0 |
| supplement8 | before | 8 | 8 | 0 | 0 | 0 |
| supplement8 | after | 8 | 8 | 0 | 0 | 0 |

## Compact16 reference provenance and residuals

All 16 resources/streams/memberships/cwd/TZ/env/pins were independently validated. Pinned Java71806a23780b04a5ccde2f8ceb5121edad5eb711, JARaa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053. Wrapper `/Users/marcelo/rustuml-reference/bin/reference --cwd <worker>/new --timezone GMT+11:00 -- -pipe -tsvg`, env{}. Worker`/Users/marcelo/rustuml-reference/scratch/worker.OREQPPhm`; Java argv derived, not process-traced. No assets added;0timeouts. Returned gzipSHA25696b90f464957fdd3b8b82e58dddd7e8f588b3ebb7bbe8cdab91d595240e7c05e; transportSHA256193bbe995671ffaaab3cbfe9b29e86704082cec5c774112fe931c108cdbd79dc. Original envelope unchanged; lossless parent mapping and original embedded metadata bytes preserved. No local Java/javac/SSH/runner or historical reference rerun.

Fifteen valid diagrams. `comment_state_links` was intended-valid but Java exits200 with `ERROR / 3 / Syntax Error? (Assumed diagram type: state)` and a syntax-error SVG. It is executable and nonpositive, not evidence about comment serialization. All other15 exit 0 with empty stderr; validity was independently classified from SVG content.

Fresh valid whole outcomes 1→11/15. Remaining failures: Unicode metrics(comment_class_unicode), empty object consumer(comment_object_links), Archimate welcome dispatch(comment_archimate), and empty-state geometry(state_alpha_empty). The latter's RGB/opacity is exact(#2468AC,0.00392), but dimensions50x40Java versus59.7578x50Rust remain wrong. No compensation or geometry edit. Every outcome/comment projection is retained in fresh-outcomes.json. Checker87comment_double_hyphen andshape_alpha pass; its83valid total34→36, with all other failures retained.

## One stable broad gate run

Workspace library: parser 624/624; renderer 797/799 (new comment test green, known plain-theme and filter-identity tests fail); other library suites pass. CLI 17/17. Product library diagnostic 11,179/11,251 and assisted 10,497/11,251 retain exact baseline 72/754 failure-name files, no gained or lost passes. These diagnostics are separate from actualCLI truth. Standing library 898/2,069 remains red; guards 4/6 remain red. Clippy stops at existing parser parse/component.rs:72deadcode. Fmt reports only existing parser/class and renderer/class residue. Diff check passes. Gates.json records exactargv/env/cwd/status/timing; rawlogs retained. No thresholds, baselines or unrelated redunits were changed. Initial evidence-tool errors and their resolution are retained in process-notes.json and rawlogs.

## Reproduction and handoff

Extract archive groups listed in archives.json into a private directory. Every archive has a SHA256, entry manifest and readback verification. Metadata includes all immutable memberships, exactcommands, source/binary hashes, scripts, parentrequest/manifest/transport validation, observations and complete losses/errors/panics. Captures retain every source/ref/SVG/YAML/stderr/comparator outcome before and after. Binary/targetbuild trees are private and excluded; rebuild the named revisions with jobs 4. Scripts retain original absolute capture paths for provenance; relocate those paths consistently when replaying elsewhere. No self-authored checkerattestation is included.

The durable residuals.json carries all explicit failures: automatic/background context, fonts/logical aliases/metrics, transparent-text wholes, gradientsT14.18, base-styleT14.19, crashpropagationT14.20, warningsT14.12, stripesT14.13, plain/codeT14.14, unconsumedlinesT14.17, sequence baselines/noteheight, forcedalignment, unknown/guillemet/URL/italic/legacytspan/Unicode/welcome and remaining negative diagnostics. Class sibling candidate remainsunintegrated; no combined correctness claim. Batik-limited math remainsnonpositive, with no math claim. Prior reviewer verdicts remain rejected. This packet awaits a new independent checker.
