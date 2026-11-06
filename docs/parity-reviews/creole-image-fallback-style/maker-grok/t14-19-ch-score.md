# T14.19 batch CH score

Scored the pre-registered predictions with `compare_svg_strict` on the whole Java stdout. Predictions were not edited.

| Case | Tip | Base |
|---|---|---|
| `hold_activity_base_bold` | match | fail, 1 diff |
| `hold_class_base_italic` | match | fail, 1 diff |
| `hold_sequence_base_bold` | match | fail, 1 diff |
| `hold_state_base_italic` | match | fail, 1 diff |

The four held-out missing-image probes match Java. The base fails each of them. Byte equality is still false because Java's non-ASCII writer and the `<?plantuml ...?>` marker are outside this comparison.

Sequence `NoteFontStyle` remains the residue from the previous measurement: `image_note_base_bold` was not in this packet. Note lines are still painted with bold, italic, and underline forced off at `sequence.rs` 15720–15730.
