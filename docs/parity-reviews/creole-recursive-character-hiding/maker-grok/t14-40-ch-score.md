# T14.40 batch CH score

Scored the pre-registered predictions with `compare_svg_strict` on the whole Java stdout. Predictions were not edited. Java results: `/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g6-creole/reference-results.json` (`aa0069bd25043719bd1669b09493724832e001eddc42319caa29bcc1b62b38d5`). All 20 Java processes exited 0.

`ref_invalid_pua` is a normal render with empty stderr, so it is a valid case. The three crash pages stay invalid.

## Valid character-reference cases

| Case | Tip | Base |
|---|---|---|
| `ref_class_eacute_uplus` | match | fail |
| `ref_class_bold_wrapped` | match | fail |
| `ref_class_combining` | match | fail |
| `ref_class_six_hex_literal` | match | match |
| `ref_note_uplus` | match | fail |
| `ref_seq_dollar_backslash` | match | fail |
| `ref_seq_emoji_five` | match | fail |
| `ref_seq_order` | match | fail |
| `ref_invalid_pua` | match | fail |
| `ref_class_eacute_amp` | fail, 1 diff | fail, 6 diffs |
| `ref_activity_dec_sep` | fail, 10 diffs | fail, 11 diffs |
| `ref_class_line_sep` | fail, 5 diffs | fail, 6 diffs |
| `ref_class_para_sep` | fail, 5 diffs | fail, 6 diffs |

Every strict match is still byte-unequal: Java writes non-ASCII text as `&#N;`.

## Residue, not fixed in this packet

`ref_class_eacute_amp`: the text atom matches, including `textLength` 68.4961. The one tip diff is the class rect fill, `#223333` against Java `#F1F1F1`. The base prediction already used `#223333`. `parse_entity_colors` (`class.rs` 2205–2251) reads `#233` out of `&#233;` and the short form expands to `#223333`. That parser is outside this lane. The failure predates the packet.

`ref_activity_dec_sep`, `ref_class_line_sep`, `ref_class_para_sep`: the converted character matches Java's decoded text (`U+2028` or `U+2029` in one text node; the class and activity heights stay one line). `textLength` is 6.6269px high at size 12 and 7.7315px high at size 14. The base failed by drawing the reference literally. The remaining gap is the face advance, which this lane does not edit.

## T14.20 crash pages

Rust exits 0. Java stderr is non-empty. The error contract is unchanged.

- `ref_invalid_amp_dollar`: `IllegalArgumentException: Illegal group reference` from `Matcher.appendExpandedReplacement`. Rust keeps the literal `&#36;`.
- `ref_invalid_uplus_surrogate`: `Invalid UTF-16 surrogate detected: d800`. Rust keeps the literal `<U+D800>`.
- `ref_invalid_uplus_nul`: `An invalid XML character (Unicode: 0x0)`. Rust emits a raw NUL, so `compare_svg_strict` cannot parse the prediction.
