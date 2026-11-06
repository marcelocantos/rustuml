# T14.19 measurement

Recorded after the production draft, against base `cb550c0d67cc729ce764895005b7e86310c9f313`. Java revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`.

## What landed

`Style.owns_font_configuration` is set only on the `AtomImg` fallback (`AtomImg.java` 104–115 and the missing-image returns at 167, 172, 179, 181, 197, 205, 209, 217, 221; `FontConfiguration.blackBlueTrue` at 63–79; `UFontFactory.monospaced` at 70–72). `effective_bold`, `effective_italic`, and `effective_underline` use that flag at every `TextBase` OR site. Ordinary atoms still OR the consumer base.

## Cohort, tip versus base

`golden_no_oracle` failure names are unchanged (70 names, 11,181/11,251). `golden_pairs` is unchanged (10,497 passed, 754 failed, 1,299 skipped, 0 panics). Perturbation failures went from 2,727 to 2,717, with no new failure.

Strict matches gained on the ae47 probes:

- `image_sequence_base_bold`
- `image_sequence_base_italic`
- `image_activity_base_bold`
- `image_state_base_bold`
- `image_class_base_bold`
- `image_title_default_bold`

`image_wbs_base_bold` and the plain-base siblings were already passing and stayed passing.

## Residual

`image_note_base_bold` still fails. The missing-image atom is plain on the base, the tip, and the pinned Java SVG. The first non-brand difference is the SVG width: Java `345px`, Rust `343px`. Note lines are painted with `bold: false`, `italic: false`, `underline: false` at `sequence.rs` 15720–15730. `noteFontStyle` is stored at `skinparam.rs` 205. `sequence.rs` is outside this lane, so sequence `NoteFontStyle` stays an explicit residual.
