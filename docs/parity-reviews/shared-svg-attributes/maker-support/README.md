# Shared SVG attributes and colors: maker evidence

This packet stops for a **new independent checker**. No maker review or acceptance attestation is authored. All prior rejections and whole-output failures remain executable.

Production: `a33dccc8e94dc44b5bafdc756fc1b88c42823a7d`. Exact start evidence `b8905e296b8865e53f3c573abaebf4adb632d751` (production `1ffb930b7b56decb15208121db5e70d252ed3c1b`). Independent rejection `84b5439aef3b41d10257d4ec964f0134a2e24ce1` was imported unchanged as `bb550b42bf7bc3bd23774df31e9b137493cb6743` before drafts. Accounts, complete classified routing list and Java serializer trace were precommitted in `d48ed1e7033e5539568a59b63c3d3229d07972db`; approved dependency/removal/state-routing addendum in `3e0642c50`; transparent-text correction account in `597f067cb`. There are no sibling production imports, target edits, baseline/comparator/skip changes, pushes or releases.

## Source mechanisms and ownership

`../java-serialization-trace.json` traces pinned PlantUML SvgGraphics DOM attribute setters → `getTransformer` US-ASCII → XmlFactories → OpenJDK21.0.5 ToStream/CharInfo. The shared encoder emits entities for ampersand, angle brackets and double quotes; apostrophe remains literal. LF/CR/TAB and non-ASCII use decimal scalar references, including one reference for a non-BMP scalar. UFontImpl's SVG family rule replaces definition double quotes with apostrophes before attribute encoding, without changing the measurement family. Invalid C0 remains unchanged under the explicitly deferred fallible export contract T14.20.

The inventory contains 3,584 dynamic sites. Parent approved 901 user-string attribute sites in 24 non-sequence files, SHA256 `1fa76a52dad7312a0745155341f1a964808ee5593c6c55ea72dbf6c3b990ac08`. Numeric/constants/allocated IDs were excluded. The routing log records mechanical producer edits; the full original list names each file, line, attribute and dataflow. The numeric/path domain census covers `d`, `points`, `textLength`, `rx`, `ry`, `font-size`: zero values outside the encoder's identity domain across 14,494 before captures. Geometry, comments, text-node content and element structure in family callsites are unchanged. The approved state base8221 exception renames only the ID placeholders to from_attr/to_attr and supplies encoded named arguments. The unused private seven-line layout_oracle escape_xml_attr was removed after all callers were routed; no declaration or caller remains in that module. No sequence, parser/model, font/metrics, native/layout or TIM file changed.

The color account is `../../shared-svg-color/account.json`. HColorSet/ColorTrieNode/XColor provide short-hex expansion, uppercase full hex, optional-hash named colors and the seven Archimate names. RRGGBBAA is split into RGB and a byte opacity; SvgGraphics.fillMe emits partial alpha/255 with five decimals. Opaque alpha omits opacity. Shape transparent fill is `none`; partial-alpha stroke follows Java's unchanged eight-digit stroke path, without invented stroke-opacity. DriverTextSvg returns before painting fully transparent text while retaining atom measurement/advance. BACKCOLOR goes through toRGB and the existing filter registry, including transparent/background black and deduplication by RGB. Automatic remains background-dependent and unresolved here.

Creole Style is a renderer-only Default/Eq/Hash struct with no serde implementation or parser-model exposure. Its optional opacity defaults to None; no unnecessary serde/model fields were introduced. All actual YAML models are semantically unchanged. Shared text APIs remain unchanged. Existing legacy tspan grammar, alias/metric resolution and base-style OR behavior are not repaired by attribute escaping.

## Actual source-only CLI evidence

Every membership, duplicate and nonpositive is retained. Private before/after release binaries use jobs 4, exact source bytes, preserved cwd/include/TZ/environment contracts, the unchanged strict comparator, and separate `--yaml -pipe`. Raw SVG, YAML, stderr, losses, process errors and panic results are archived. Library diagnostics below are a separate path.

Full golden census: **12,550 inputs**, 11,251 eligible references, **11,179 → 11,179 CLI passes**, zero losses/new errors/new panics. The CLI emitted 12,470 SVGs; 80 inputs retain process errors/no SVG, and are not XML successes. There are zero malformed emitted golden SVGs. Only **16** golden SVGs changed bytes: **14 pass → pass**, **2 fail → fail**, zero newly passing/failing. All 16 are XML-equivalent attribute re-encodings with identical decoded values, text and structure. `changed-goldens.json` names each. Fourteen changed attribute spellings match Java bytes exactly; the Hindi/Thai failures have pre-existing decoded ID differences from Java, so only compliance with Java's encoding rule—not an exact byte-distance improvement—is claimed for those two.

| Cohort | Entries | Valid | Before CLI pass | After CLI pass |
| --- | ---: | ---: | ---: | ---: |
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
| fresh16 | 16 | 16 | 14 | 15 |
| golden-regressions | 6 | 6 | 6 | 6 |
| checker36 | 36 | 34 | 15 | 19 |
| fallback16 | 16 | 14 | 11 | 13 |
| checker32 | 32 | 29 | 17 | 19 |
| shared22 | 22 | 18 | 4 | 10 |

The prior 18 cohorts total **2,082 entries / 1,941 valid**, **974 → 983** passes. Fresh compact22 is **4 → 10 / 18 valid**. Combined 2,104 memberships retain all 145 nonpositives: **978 → 993** passes. There are **zero lost passes, new errors or new panics** across replay and full goldens; the existing 32 sequence panics remain failures. The six monospace controls stay 6/6. Every replay/fresh YAML is byte-identical. Nine full-golden YAMLs differ only in map order (eight DOT cases and sprite_multiple); tagged decoded models, UIDs, identity, content and object counts are identical. Complete before/after YAML and named changes are preserved; no semantic model losses occurred.

## Fresh provenance and classifications

`fresh-outcomes.json` classifies every exact source independently. All22 Java calls exit0; 18 are valid diagrams. The three C0 tooltip controls contain SAXException crash SVGs plus matching stderr. font_invalid_color contains NoSuchColorRuntimeException crash SVG plus matching stderr. These four are **nonpositive**, never passes inferred from exit status. Original request/manifest/files envelope, resources, mapping and parent validations are untouched and archived. Worker `/Users/marcelo/rustuml-reference/scratch/worker.pI7RF50I`; raw TZ GMT+11:00, env empty, exact wrapper/cwd calls. Java argv is derived from the pinned wrapper, not process-traced. No local JVM, SSH, new runner or previous-batch rerun was used.

Result gzip SHA256 `7482c14d6e7ff931beeeedf8cabe5dd7b5b36ed549d2b5c5accd7cac594fa2b3`; transport SHA256 `fd3b76ab653a4f3c06b6d665812f9bfc6fcaea34fb669134675fc6f2c383163d`. Reference Java `71806a23780b04a5ccde2f8ceb5121edad5eb711`, JAR `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`. The validator checks source/stream hashes, resource memberships, transport, argv, cwd, TZ, environment and pinned provenance; zero timeouts. No global historical font/Graphviz equivalence claim is made.

Java splits the Unicode URL control at NBSP; that case does not establish full Unicode URL preservation. fontUnicode independently exercises Unicode attribute encoding. Shared alpha/BACKCOLOR atom values match their reference controls while the two note cases still fail whole output on sequence note-height rounding. attr_actor_metadata is now well-formed but retains qualified-name normalization differences. All URL/label, automatic and other whole failures remain recorded.

## Standing XML oracle and dependency scope

`crates/rustuml/tests/pipe_mode.rs::every_cli_golden_and_registered_perturbation_svg_is_strict_xml` runs the actual CLI over every golden and registered perturbation pair, parses every nonempty emitted stream regardless of reference validity/exit, and reports errors/panics separately. It is wired into the existing pipe_mode target already invoked by test-parity. The test-only roxmltree0.20 parser uses strict XML parsing with only allow_dtd=true. Its source `parse.rs:315–345` describes the default DTD rejection as optional policy. The test guards exactly26 standard SVG1.1 PUBLIC DOCTYPE declarations from DOT sources; all other options and internal entity-expansion limits remain defaults. Pinned tokenizer source consumes external IDs and discards external entity definitions; **no external DTD/entity is fetched or resolved**. The initial CLI15/16 DTD-policy failure and corrected affected-test-only rerun are both retained.

The expected-malformed set is exactly the three approved attr_link_c0_one/zero/escape sources, each keyed to its full SHA256, owner T14.20 and reason. Unlisted malformed output, stale source hash, missing expected source or a listed source becoming well-formed all fail the test. These three outputs remain **malformed Rust XML**, despite the expected-failure ratchet test passing. Current census: goldens12,550/emitted12,470/malformed0/errors80/panics0; registered perturbations1,966/emitted1,857/malformed3/errors109/panics32. The previous1,944 registered pairs had three malformed quoted-family outputs; all three are now well-formed. `xml-by-cohort.json` provides malformed counts for every retained checker/maker cohort, including nonpositives.

Dev dependencies are roxmltree0.20 and sha2 0.10 (resolved0.10.9). Exact `Cargo.lock.diff` adds seven SHA support nodes and two dev edges; roxmltree0.20 was already locked. `cargo tree -p rustuml -e normal` is byte-identical before/after. No production dependency edge changes.

## Gates and residuals

The stable production broad gates ran once. Workspace library: parser624/624; renderer796/798, with the same two known failures (class plain-theme defaults and filter source identity). Five focused new semantic tests pass. Existing explicit_font_monospace_inherits_surrounding_metrics remains green with its unchanged expectation. CLI initial15/16 has only the DOCTYPE-policy test failure; its approved correction passes the affected test alone, covering all14,516 inputs. Other15CLI tests passed in the original run.

Product library diagnostic **11,179/11,251**, assisted **10,497/11,251**. Failure-name files are byte-identical to the exact baseline:72 and754 respectively, no lost or gained library passes. Standing perturbation library diagnostic **851/1,966**, still red; guards4/6, still red. Broad clippy stops on known parser dead code; a scoped diagnostic confirms only existing warnings, no new warning in changed code. Formatter retains existing parser/class formatting residue. `git diff --check` passes. No unrelated red-unit fixes or threshold changes were made. Initial draft failures, borrow-correction compile logs, failed focus assertion, initial transparent emission observations and failed DTD gate are retained.

Explicit unresolved obligations are detailed in `residuals.json` and `../sequence-handoff.json`: automatic background plumbing; sequence local attribute encoder coordination, mixed-size baselines, note height and stripe classification; warning chrome; forced alignment; unknown/guillemet, Unicode/font/PLAIN, note resolver, URL/italic/legacy tspan, welcome and remaining negative diagnostics. Malformed DOT, size zero/overflow, font-color throws and C0 export exceptions remain T14.20. Math controls remain Batik-limited nonpositives with no math claim. Font alias/whitespace/logical metrics, gradients, consumer base-style OR and unconsumed lines remain separate targets. Class generic/positioned-stripe sibling work is unintegrated; original markup_literal/markup_scan/style_nested obligations remain, with no combined correctness claim.

The comment inventory has45 user-string emissions,4 generated markers and1 non-emitting search candidate. No comment content changed. Before/after golden, registered and retained/fresh capture scans found **zero malformed comments**, including the explicit -- and trailing-hyphen rules. This is corpus evidence, not a universal proof that arbitrary future user comments are safe.

## Reproduction and archives

`archives.json` lists every archive hash, entry manifest and verified readback. Extract all archives into a private cache preserving relative names; metadata contains the exact scripts/commands, memberships, raw relay envelope, stream hashes and binary/source hashes. before/after binaries remain private at `/Users/marcelo/Library/Caches/rustuml-shared-svg-maker/`; binaries and build targets are not committed. Commands are captured in each result and final-gates.json; `full_cli.py`, `run_replay.py`, `registered.py`, `analyze_outputs.py`, `model_compare.py` reproduce the observations with the unchanged probe source.

Historical verification compared **11,031** pre-existing review/account/perturbation files against imported-rejection tree bb550b42 using git cat-file: every byte identical. The original rejection files independently match84b5439 unchanged. No review.json or checker attestation was authored for this packet. A new independent checker must attempt to disprove the claimed mechanisms.
