# Sequence participant and colour residue: maker results (corrective round)

Maker: Claude Opus 5.5. This is a maker record, not a verdict. Account: `account.json`, commit `65b5a1633`. Production: `ecbd75ec8` and `87bdb0feb`. Same before/after trees and cohorts as the sprite mechanism, which records the gates.

## Implemented (Java trace)

- **URL tooltip.** `CommandParticipant#executeArg` → `UrlBuilder#getUrl`, and `Url#getTooltip` is the anchor title. `Participant.url_tooltip` is recorded and used as the legacy `title`/`xlink:title`. This adds 8 whole matches.
- **Created participant head.** `ArrowAndParticipant#drawInternalU` calls `drawParticipantHead` with no `startUrl`; only `DrawableSet#drawHeadTailU` opens the link. The head is now drawn without the anchor, which adds 1 match (`c75_create_link_legacy`).

## Explicit warnings (no longer silent)

- Multiple stereotypes: `Stereotype#build` splits them into one label each, but Rust renders a single label.
- A `[[link]]` inside a quoted participant display (a creole link in Java).
- Teoz participant anchors (`LivingSpace#startUrl`) are not emitted.
- A re-declaration that `SequenceDiagram#putParticipantInLast` would reorder or move into another box (`addWithOrder` and englober).
- The `CommandGrouping` COLORS[1] colour (`backColorGeneral`) is not painted.

## Fresh counts (`fresh-participant-color-counts.json`)

- complete-consumption: 81 match (72 + 9 gained). 123 mismatches now all warn, and 0 are silent; before the change 42 were silent. 72 Java-invalid inputs stay rejected.
- residue-preservation: all 42 mismatches warn.
- command-specific colour paths: 63 match, 131 warned mismatches, 179 rejected, **23 silent mismatches**. `color-paint-audit.json` compares the multiset of fill and stroke colours between Java and Rust for those 23:
  - In 17, the paint is identical. The differences are note-across, over-several, hnote/rnote and box geometry, not colour.
  - In 6 return probes (`return #PaleGreen/Indigo pollen` and variants), `CommandReturn` COLOR is `(#\w+)` followed by whitespace, so the text is MESSAGE. Java renders it as a creole ordered-list item ("1."). That is a creole residue, not a colour path.

  No warning keyed to label text was added, because the parity rules forbid it. `group_duplicate` now warns.

## Residue

- Multi-stereotype rendering.
- Teoz anchors.
- Re-declaration reorder and signed order: warned, not modelled.
- Note/hnote/rnote across and over-several geometry.
- Return-label creole lists.

The batch2 request in `pending-request/` (a byte-identical copy, see `SHA256SUMS`) includes colour-free controls (`mk2_note_across_plain_legacy`, `mk2_hnote_over_plain_legacy`, `mk2_rnote_across_multi_plain_legacy`, `mk2_hnote_right_plain_teoz`, `mk2_box_plain_teoz`). They test whether the geometry divergence is colour-independent. The request also includes `mk2_group_space_colour_teoz`, `mk2_redeclare_into_box_*`, `mk2_multi_stereo_three_*` and `mk2_tooltip_in_box_*`.
