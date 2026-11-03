# Independent adversarial TIM review

All four mechanisms are rejected. This review used inherited Codex intentionally
(`gpt-6-astra`, high reasoning), without children. Production was not changed.
The combined tested revision is `9102a45ed27188abf9f0db7784daed490a48ec1c`,
the seven requested cherry-picks over `bb2bdb67d5efc780de2c04f788dbbe8d53b54196`.
Each mechanism's adjacent `review.json` identifies original implementation and
follow-up commits, rather than pretending the cherry-picked hashes were the
maker's original history. Only review evidence is delivered by the final commit.

## The source-reader discrepancy is explained

The frozen three source/SVG pairs are byte-identical to their versions in RustUML
commit `97ac6290982538fb893edc03f10f34cd714e63b2`. That tree contains
`test-diagrams/golden/gen_activity.py`, `gen_edge_cases.py`, and `render_all.py`.
The current golden submodule is `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`;
its initial pair-import commit is `e0e2cf6a`. Exact blob IDs and SHA256 values are
in `provenance-results.json`.

The activity generator's `wrap` uses `body.rstrip()`, preserving the leading LF
in its triple-quoted body. Its two shadowing loops therefore write an empty line
after `@startuml`. The edge generator writes its three leading and internal
empty lines explicitly. The generator does not strip those leading lines.

The historical `render_all.py::render_one` reads the entire source as UTF-8.
`plantuml_encode` deflates the entire string (`zlib.compress(...)[2:-4]`) and
encodes it with PlantUML's alphabet; it does not trim the diagram. It requests
`GET /svg/<encoded>`. The exact historical encoder was imported from its Git
blob for this reproduction, with all network output directed to scratch or
this intentional review-support directory.

At pinned Java `71806a23780b04a5ccde2f8ceb5121edad5eb711`:

1. `picoweb/PicoWebServer.handleGET` calls
   `TranscoderUtil.getDefaultTranscoderProtected().decode`.
2. `code/TranscoderSmartProtected` uses `TranscoderImpl` with
   `DiagramSourceCompressor` and protected zlib decompression.
3. `TranscoderImpl.decode` decodes/decompresses bytes, then calls
   `DiagramSourceCompressor.decompress`.
4. `DiagramSourceCompressor.clean` calls `StringUtils.trin`, `clean1`'s regex
   extraction, removes UML delimiters, and trims again. `decompress` then
   reconstructs `@startuml\n` + cleaned body + `\n@enduml`.
   The regex's whitespace around the body consumes initial and final blanks.
   Internal blanks remain. `reader-contract/*/decoded.puml` contains the exact
   returned bytes, obtained from the actual Java decoder, not a Python model.
5. Only then does GET construct `SourceStringReader`. POST `/render` passes the
   source directly. `Pipe.readSingleDiagram` appends every line after the start
   marker; `managePipe` also constructs `SourceStringReader`. The file path and
   string path ultimately use `BlockUmlBuilder`/`Preprocessor`, retaining located
   blank lines. `UmlSource.loadInternal/getPlainString/seed` preserves them in
   the identity; this is downstream of GET's transformation.

Fresh results with identical raw input bytes:

| Input | Frozen / GET | Pipe / file / POST / candidate |
|---|---|---|
| `act_skinparam2_shadowing_true` | filter `f4a86o8ehqhok` | `fuon9sr0qvzo4` |
| `act_skinparam_shadowing` | filter `f10sykl8jbxrix` | `fj5dajj5vm98p` |
| `edge_whitespace_multiple_blank_lines` | origins 1, 5, 9 | 4, 8, 12 |
| Exact parser assertion and renamed counterpart | origins 1, 4, 5 | 5, 8, 9 |

For all three frozen pairs, fresh GET is **exactly equal after XML parsing** to
the frozen reference, beyond merely passing the existing comparator. Feeding
the actual decoded bytes to pipe reproduces GET. Pipe, POST and file agree after
XML parsing. The shipped Rust CLI strictly matches the raw-input pipe output
and fails the frozen GET output. The parser assertion's exact original bytes
and a renamed perturbation were both exercised through all four paths. The
assertion entered the integration history in `b3eb1bcbd138314ece69a0b23ea2191b19008b79`
(also present in WIP snapshot `2083406b53577978cd77b7e7d7a077bb1c1ab4be`).
Its expectations match GET normalization, not direct Java parsing. The historical
author's intent is not recorded here and is not inferred.

This is a proved front-end normalization difference, not an unexplained Java
API mismatch or a font/Graphviz hypothesis. Per-file historical HTTP access logs
are unavailable; historical route attribution is supported by the checked-in
renderer, unchanged blobs, and exact reproduction. The rebuilt JAR still is not
historically byte-identical and broad font/Graphviz equivalence is unproven.
The private picoweb process was terminated in `finally`; no shared server or
reference checkout was modified.

## Decisions requiring owner sign-off

No alternative below was implemented, and none waives the other counterexamples.

* **Raw Java CLI/file contract:** keep blank lines in the evaluated stream.
  With explicit approval, change the parser assertion to 5/8/9 and introduce a
  provenance-declared raw-input reference tier. Preserve the historical GET
  corpus as historical evidence. A narrowly audited replacement or additional
  raw reference for affected cases is a reference-policy decision, not routine
  baseline maintenance. Never quietly normalize raw product inputs using SVG
  contents or label-specific conditions.
* **Historical GET compatibility contract:** explicitly model Java's
  `DiagramSourceCompressor` at the reader boundary, before TIM, and preserve
  the existing frozen references/assertion. Apply the named transformation
  consistently (delimiters, outer whitespace and internal preservation), with
  a separately specified raw CLI mode if desired. This deliberately chooses
  different raw-input semantics from Java pipe/file; it requires sign-off.
* **Defer the contract change:** retain the parent until the contract is
  decided. Extracting corrected memory/lookup/date changes may reduce coupling,
  but each requires a fresh isolated review and its current counterexamples
  remain blockers. No mechanism is accepted merely because the aggregate's
  reference conflict now has an explanation.

The minimal policy repair is to name the front end in the reference contract.
The minimal code repair then belongs at that boundary, not in layout constants,
SVG IDs, comparator slack, or per-fixture logic.

## Mechanism findings

* **Evaluated events:** ordinary themes, subblocks, sprites and identity
  projections show real gains. Nevertheless `eval_function_call` discards the
  `Vec<EvaluatedEvent>` returned by `call_function`, and inline expansion filters
  out non-Plain events. Java's `TContext.resultList` receives executed themes
  regardless of whether a return value is requested. `checker_return_function_theme_event`
  loses the theme and changes the SVG. `checker_selector_nested_theme_event`
  loses its first theme from identity even though its SVG happens to compare
  equal. `try_silent_directive` also discards `!log` before evaluating its argument;
  Java `executeLog`/`EaterLog.analyze` calls `applyFunctionsAndVariables`.
  `checker_diagnostic_side_effect` renders 0 instead of 1. These are direct
  execution-model failures, independent of the historical reader decision.
* **Procedure memory:** current-frame override/global/automatic-local lookup,
  writes, removal, sibling calls and branch/foreach restoration pass five
  adversarial controls. `checker_default_capture` fails: Java
  `Eater.eatDeclareFunction` evaluates and stores a default `TValue` when the
  declaration is read; `TFunctionImpl.getNewMemory` binds that value. Rust
  `parse_func_params` stores raw `$seed`, and `call_function` clones that text.
  Java renders `first`; Rust renders `$seed`. This is a retained pre-existing
  default-binding defect, not a claim that the sibling-frame change introduced
  it. It still falsifies complete parameter-memory parity.
* **Lookup:** direct `"$" + "cedar"` is wrongly treated as one quoted literal
  by `strip_prefix('"').and_then(strip_suffix('"'))`; single-quoted names undergo
  the wrong textual substitution. Java `TokenType.eatOneToken`, `TokenStack`
  and `Eater.eatAndGetQuotedString` parse expression tokens and both quote
  forms. Java renders `1 / green` and `1 / silver`; Rust renders `0 / `.
  Legacy bare/dollar collisions, nested scope and variable-stored computed
  names pass, so the defect is not a blanket rejection of all lookup work.
* **Explicit date:** signed i32 minimum/maximum, negative and zero boundaries,
  arithmetic timestamps, and 12 live host-zone checks pass. Java
  `DateFunction.executeReturnFunction` calls `TValue.toInt()`, multiplies by
  `1000L`, and delegates to `SimpleDateFormat`; the new signed conversion and
  instant-specific timezone lookup have positive evidence. The existing Rust
  formatter does not implement `Z/z/XXX`, single-width fields, or doubled
  apostrophes. Those valid format counterexamples reject complete date parity.
  Deliberately invalid overflow and `QQ` controls additionally expose silent
  output: Java rejects, Rust exits 0. The overflow reference stdout is empty;
  that exact empty `.svg` and Java stderr are retained, never fabricated into
  a valid SVG. These invalid controls are excluded from valid-positive counts,
  but are not excluded from the executable held-outs.

The source audit found no new fixture-label conditions, golden reads from
shipping crates, coordinate surgery or comparator changes. New calendar and
hash constants have named Java/Gregorian provenance; the identified failures
are semantic paths, not tunable tolerances. Silent include/unknown-builtin
fallbacks already existed; the new explicit-date parse failure also returns an
empty string rather than preserving Java rejection. No production fix was made.

The four account files additionally use `claimed_invariant` and
`planned_model_change`, while the unchanged guard requires `invariant` and
`planned_change`. This schema mismatch remains visible; this checker is not
authorized to edit maker accounts.

## Evidence and reproduction

`check.py` drives the shipped release CLI and the original maker comparator
probe, linked against this checkout's unchanged oracle library. It records
Rust SVG/metadata, fresh Java CLI SVG, source identity diagnostics, stderr and
exit status. `ReaderProbe` is used for identity/decoder diagnostics only;
all reference SVGs come from the actual CLI or specified HTTP/file experiment.
`case-manifest.json` distinguishes novel cases, five retained valid maker failures,
the exact assertion control and deliberately invalid checker controls.

Prepared corpus: 913 sources, 400 isolated CLI matches, 503 mismatches, eight
invalid references, two genuine panics. Built-in package-cwd replay: 399 matches,
504 mismatches, eight invalid references, two panics. The sole difference is
`checker_include_theme` (T14.6.2). Both nested self-activation panic cases remain;
registry cleanup prevents collateral failures. `prepared_builtin.py` temporarily
parks only this checker's four new fixture directories, restoring them in
`finally`, to measure the original 913 independently of additions.

Independent fresh replay confirms the maker's preserved 33: 28 SVG matches,
four valid mismatches, one invalid reference; all 33 identity streams match,
including the invalid case. Maker 22: 19 SVG matches among 20 valid inputs,
20 exact valid source matches, two invalid controls. Both maker invalid controls
remain in their original locations and are not promoted to valid held-outs.
All five relevant valid failures are additionally retained with fresh references
in this checker's executable event-stream directory.

Parser: 569 pass, one unchanged assertion fails. CLI pipe tests: 13 pass.
Main product: 11179/11251; assisted: 10497 pass, 754 fail, 1299 skipped.
Both main tiers are red. The full final held-out and guard results, additions'
separate counts and artifact hashes are in `verification-summary.json`.
Guards remain red; successful schema construction here does not certify
historical records or authorize acceptance.

Environment: `CARGO_TARGET_DIR=/tmp/rustuml-tim-checker-d2205b7f/target`,
`CARGO_BUILD_JOBS=4`, `RAYON_NUM_THREADS=4`, pinned `PLANTUML_STDLIB` and
`RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100`. Absolute Temurin 21 Java and
the source-pinned JAR are recorded in `reference-environment.json`. New canonical
date references explicitly use Java `GMT+11:00` to match the gate's fixed offset;
the `z` mismatch is also tested alongside numerical `Z/XXX` and cannot be
explained away by a timezone display-name difference. Supplemental timezone
runs unset the debug zone and set both `TZ` and Java `user.timezone` to the same
IANA zone. Their full source/SVG pairs and results remain in date checker support.

Run `reader_contract.py` and `provenance.py` for the reader trace;
`check.py isolated` for the prepared CLI inventory; `replay_maker.py` for both
maker groups. `case-manifest.json` is the complete final manifest; `check.py fresh`
replays it. `prepare` is only the initial generator and is not a replay step:
`timezones.py`, `error_controls.py`, and the reader assertion experiment extend
that manifest. The held-outs are wired automatically to the existing
`parity_perturbations` standing gate. Logs, including failures, are preserved.

`git diff --check` reports whitespace in captured Java/Rust theme streams,
comparison output and the deliberate whitespace input. Those bytes are evidence
and are intentionally retained. The initial aggregate log also records a
temporary missing reference while the first new reader pair was being generated;
its held-out count is superseded by the completed prepared-only and final runs,
not treated as a product regression or hidden. No tests or gates were weakened.
