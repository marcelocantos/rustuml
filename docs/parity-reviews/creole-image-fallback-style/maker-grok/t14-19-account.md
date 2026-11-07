# T14.19 maker account — AtomImg owns its font configuration

Precommitted before the production draft. Base `cb550c0d67cc729ce764895005b7e86310c9f313`. Java revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`.

## Java trace

`AtomImg.create` (`klimt/creole/atom/AtomImg.java` lines 104–115, and the missing-image returns at lines 167, 172, 179, 181, 197, 205, 209, 217, and 221) builds a new configuration and passes it to `AtomTextUtils.createLegacy`:

- `UFontFactory.monospaced(14)` (`klimt/font/UFontFactory.java` lines 70–72): family `"Monospaced"`, `UFontFace.normal()`, size 14.
- `FontConfiguration.blackBlueTrue` (`klimt/font/FontConfiguration.java` lines 63–65) calls `create`, whose `getStyles` (lines 67–79) reads the face. A normal face yields an empty style set: not bold, not italic, not underlined.

That configuration replaces the stripe's `FontConfiguration`. It does not copy the consumer's bold, italic, or underline. Creole commands that do inherit call `add` / `bold` / `italic` / `remove` on a copy (`FontConfiguration.java` lines 301–347). The image fallback is not one of those copies.

## Precommitted measurement

Recorded on this base by `review.json` in this directory (checker ae47), not re-derived from coordinates:

- `image_sequence_base_bold`, `image_activity_base_bold`, `image_state_base_bold`, `image_class_base_bold`: Java fallback is plain. Rust adds only `font-weight="700"` on the missing-image atoms. Text, size, color, `textLength`, and coordinates otherwise agree.
- `image_sequence_base_italic`: the same shape, with italic instead of weight.
- `image_title_default_bold`: title text is default-bold; the fallback fails only by `font-weight="700"`.
- `image_note_base_bold`: the fallback atom is plain on both sides. Java draws the surrounding note text bold. Rust draws it plain, so the note is narrower and the x positions move. `review.json` attributes that to sequence note configuration, not to the image atom.
- `image_wbs_base_bold`: `defaultFontStyle` bold did not activate in Java or Rust. The pass does not show a base-bold reset.

Rust cause, still present at this base: `text_render.rs` computes `base.bold || style.bold` and `base.italic || style.italic` in `trim_text_for_emit`, `text_width_for_style`, `tab_stop_width`, and `write_text_element`, and `style.underline || base.underline` in `write_text_element`. The image `Style` built in `creole.rs` `handle_tag` already clears the nested flags, but the OR puts the consumer base back.

## Planned change

Add `Style.owns_font_configuration`, default false. Set it only on the `AtomImg` fallback style (monospace, size 14, fill `#000000`, other flags clear). When the flag is set, measurement and emission use the segment's own bold, italic, and underline. Otherwise the existing OR remains. All four bold/italic sites and the underline site share one helper, so width and paint cannot diverge.

No geometry constant. No change to ordinary creole atoms.

## Residual, not in this draft

Sequence `NoteFontStyle` is stored at `skinparam.rs` line 205 (`noteFontStyle` → `t.note.font_style`). Note lines are painted with `bold: false`, `italic: false`, `underline: false` at `sequence.rs` lines 15720–15730. `sequence.rs` is outside this lane. `image_note_base_bold` stays a failure for that reason. The image atom on that probe is already plain and is not the residual.

## Axes

- consumer base: sequence message, activity, state, class attribute, and default-bold title, each with a missing image
- face: base bold and base italic, held out from the ae47 wording
- neighbor: text before and after the image keeps the consumer base
- non-effect: plain-base siblings and the WBS case stay as they are
