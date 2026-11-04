# Recovery4 remote36 follow-up: maker evidence only

The immutable pending snapshot is commit `4419cf130ab8c4da3e81e4a29595c4dc801b42df`; its README, all original sources, Rust captures and pending statuses are unchanged. This separate follow-up supplies the returned Java references and evaluates the already-preserved Rust captures from production revision `7bd5c9f5d2f4dee23472ddc8e3ac4b96a3e196e3`.

**28 of 36 match; 8 valid-reference mismatches remain. No acceptance claim.** No input is invalid XML or marked erroneous by the unchanged `golden_has_syntax_error` classifier. All 36 Java calls exited zero, but validity and comparison were checked separately; zero exit alone was not treated as parity. All reference stderr streams are empty.

## Transport and source identity

Parent delivered the remote results through the T3 relay. Worker: `/Users/marcelo/rustuml-reference/scratch/worker.LYsaMBCc`. Input compressed SHA256: `b29bba1ee4dcf64a155f8a7dabd2a2aca2723f5023ce4092a65cd7dad88ccb93`. Returned compressed SHA256: `53ec8058bcc4db68c5746d53834670df0b063c9cebfd611099011044386b1732`.

The raw `.json`, original `.json.gz`, transport JSON and source manifest are preserved byte-for-byte here. The transport base64 decodes exactly to the compressed file, and decompression equals the raw JSON bytes. All 36 unique source memberships and source hashes match the immutable snapshot; every decoded stdout/stderr hash matches its remote record. Every retained Rust SVG hash matches the pending snapshot. No source trimming, newline normalization, label substitution, reference regeneration or SVG surgery occurred. Spaces in original filenames are retained.

The bundle records each exact wrapper and Java argv, private cwd, explicit `GMT+11:00`, `-Djava.awt.headless=true`, UTF-8, locale, elapsed time, exit, and zero timeouts. Java source pin: `71806a23780b04a5ccde2f8ceb5121edad5eb711`. JAR SHA256: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`. Remote OpenJDK 21.0.5 and Graphviz 12.2.0 differ from the original environment; prior seven-case calibration was scoped, not global equivalence. The source-confirmed failures below are not waived as environment differences.

## Every failed input and bounded diagnosis

All paths below are relative to `pairs/fresh-controls/`. Each has exact `.puml`, returned `.svg`, original `.rust.svg`, `.java.stderr`, comparator `.diff.txt` and `.diff.stderr` files. `comparison.json` contains all 36 statuses, commands, hashes and diff text; `observations.json` records the bounded XML measurements.

| Failed input | Java / Rust observation | Causal scope |
|---|---|---|
| `boundary_18_False.puml` | Canvas height 180 / 183; label baseline 164.4023 agrees. | Preferred height incorrectly used as painted maximum. |
| `boundary_18_True.puml` | Canvas height 202 / 204; label baseline 185.6016 agrees. | Same defect with stereotype present. |
| `header_0_15.puml` | Circle center/radius/fill agree; custom `Q` lacks final `Z`. | Shared glyph contour serialization. |
| `header_0_9.puml` | Circle center/radius/fill agree; custom `Q` lacks final `Z`. | Same at alternate radius. |
| `header_4_15.puml` | Circle center/radius/fill agree; custom `Q` lacks final `Z`. | Same with padding. |
| `header_4_9.puml` | Circle center/radius/fill agree; custom `Q` lacks final `Z`. | Same across both axes. |
| `modifier_bottom.puml` | Expected entity `<g>` containing `<a>`; actual `<a>` containing entity `<g>`. | URL/group ownership in note serialization. |
| `modifier_left.puml` | Same inverted nesting for a braced note. | Same across position and body delimiter. |

### Boundary font size: preferred versus painted maximum

Java `decoration/symbol/USymbolSimpleAbstract.asSmall.drawU` places the label after the symbol and stereotype, while its `calculateDimension` returns their preferred block dimensions. `klimt/drawing/LimitFinder.drawText` (lines 203–210 at the pinned source) uses the text baseline: the painted maximum is `baseline + 1.5`. `svek/SvekResult.calculateDimension` (lines 130–136) measures that drawn image and adds its 15px envelope.

Rust `crates/rustuml-render/src/class.rs:974` correctly computes the preferred boundary height from 32px symbol height plus label/stereotype line heights, but `render_plantuml_svg` at lines5203 and5391 also uses `y + dims[i].height` for body/canvas maxima. The painted text maximum may be smaller than the preferred line box. In these captured cases, `floor(label_baseline + 1.5) + 15` reproduces both Java canvas heights exactly, while all interior elements match under the unchanged comparator. This is a source-derived frontier distinction, not a proposed per-input subtraction.

The size 12 controls expose the same smaller error: `boundary_12_False` advertises 175px versus 176px, and `boundary_12_True` 189px versus 190px. They count as comparator matches solely because the existing root SVG policy allows ±1px (`compare.rs:517–541`). Their interior geometry also agrees. These matches do not establish exact boundary canvas equivalence. No comparator tolerance was changed.

The next implementation, if assigned, needs an earlier causal account for the final painted maximum of simple symbols, preserving their preferred dimensions for layout. This follow-up does not implement it.

### Custom Q: final contour closure, not circle centering

All four header controls have identical ellipse attributes in Java and Rust. Java `CircledCharacter.drawU` delegates the glyph to `UCenteredCharacter`; `DriverCenteredCharacterSvg.draw` passes the AWT outline to `SvgGraphics.drawPathIterator`, which serializes every `PathIterator.SEG_CLOSE` as `closepath()` (lines 980–1004).

Rust `class.rs:9483` routes this custom character through `metrics::centered_character_path`. In `crates/rustuml-render/src/metrics.rs:318–373`, closure is inferred only for a closing `OutlineCurve::Line`. The inner `Q` contour finishes through a quadratic curve, and the function returns without appending a close marker. Java emits two `Z` commands; Rust emits one. There is also an observed 0.0001 coordinate rounding difference, within the frozen geometry epsilon; the missing path command is structurally different and remains a failure. No glyph is replaced with a captured path.

This reaches the shared font/glyph serializer outside the current class-only production scope. It is an explicit ownership dependency for a future generative contour repair, not a request to waive the four controls. Earlier measured circle-center invariants alone were insufficient to establish full header parity.

### Note URL/group ordering

Java `svek/image/EntityImageNote.drawU` creates the entity group at line 200, opens the URL at 203, draws the image, closes the URL at 265, then closes the group at 267. Rust `class.rs:14965` and `:15697` open `note_anchor` before emitting the entity group; the attached-note path closes the group before the anchor at 15129. The fresh left/braced and bottom/end-note controls therefore preserve command-looking body text, note metadata and geometry, but invert the DOM ownership of the URL and entity group. The frozen comparator correctly rejects both tag-order differences.

A future correction belongs in the owned note rendering paths under a new precommitted account. It is not a parser/body-ownership failure and does not authorize changing parser fallback or dropping URL metadata. No correction is made in this evidence-only follow-up.

## Passing inputs and limits

The 28 matches comprise four actor shadow/direction controls, two boundary size 12 controls (with the above 1px canvas caveat), four member icon/padding controls, four selected-symbol/alignment controls, four package visibility/style controls, two quark declaration-order controls, four blank/content ribbon controls, and all four single/linked shadow controls. The entire list remains in `comparison.json`. These are maker cases, not fresh independent checker acceptance.

The frozen 1192, parent 1449 and previous 1559 measurements are unchanged historical scopes; this is a separately measured 36-input addition, not a rerun or a newly certified combined denominator. Earlier residuals remain visible in recovery4. No full suites, new Rust renders, local Java, local SSH, child agents, production changes, comparator edits, baseline changes or shipping actions were performed.

## Reproduction and durable contents

`evaluate.py` verifies bundle/hash identity, decodes exact streams, classifies XML, and invokes the existing unchanged comparator on each returned reference and preserved Rust capture. Its absolute cache paths are intentional provenance; copy the script into the maker's `remote-followup` cache to rerun. `diagnose.py` only reads XML and emits bounded measurements; it does not rewrite SVGs. `comparison.json` records the precise comparator commands and binary hash, while `provenance.json` records its source hash. The original comparator build recipe remains in recovery4.

The pending snapshot and all prior evidence remain intact. This follow-up is evidence only. Eight fresh valid counterexamples remain failures pending a separately assigned repair and fresh independent checker.
