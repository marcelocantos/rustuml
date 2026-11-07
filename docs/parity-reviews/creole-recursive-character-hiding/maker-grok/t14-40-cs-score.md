# T14.40 batch CS score

The held-out batch staged as CN was relabelled CS. Scored the pre-registered predictions with `compare_svg_strict` on the whole Java stdout. Predictions were not edited. Java results: `/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g6-cs/reference-results.json` (`83d3cdd7b7d0baacefb1b8e2258727f6d9b21dcb729e5020aaa2fa8e30acfcc1`). Predictions copy: `rust-predictions.json` (`68f1c2bde95533d9f6960c29f3fe7b1b9c67ed156153e29998edaae67afc2d58`). All 25 Java processes exited 0 with empty stderr and a class, sequence, activity, or state SVG, so every case is valid.

Tip is `3f6bf7a7332da0ad85dbc514d4af665f82920632`. Merge base is `8d294442f4d88aed3f1c353a9766045fc90f2a7c`.

## Per case

| Case | Tip | Base |
|---|---|---|
| `cn6_activity` | match | exit 1, empty stdout |
| `cn6_before_pua_end` | match | exit 1, empty stdout |
| `cn6_below_escape` | match | match |
| `cn6_bold_last` | match | exit 1, empty stdout |
| `cn6_carrier_first` | match | exit 1, empty stdout |
| `cn6_carrier_mid` | match | exit 1, empty stdout |
| `cn6_escape_scalar` | match | fail, 3 diffs |
| `cn6_last_scalar` | match | exit 1, empty stdout |
| `cn6_mix` | match | exit 1, empty stdout |
| `cn6_nonchar` | match | exit 1, empty stdout |
| `cn6_note` | match | exit 1, empty stdout |
| `cn6_packed` | match | fail, 3 diffs |
| `cn6_plane1` | match | match |
| `cn6_plane14` | match | match |
| `cn6_plane2` | match | match |
| `cn6_pua_a` | match | match |
| `cn6_pua_b_end` | match | exit 1, empty stdout |
| `cn6_pua_b_low` | match | match |
| `cn6_rocket` | match | match |
| `cn6_state` | match | exit 1, empty stdout |
| `cn6_uplus_five` | match | match |
| `cn6_uplus_max5` | match | match |
| `cn6_uplus_six_first` | match | match |
| `cn6_uplus_six_last` | match | match |
| `cn6_uplus_six_pua` | match | match |

Tip: 25 matches, 0 diffs, 0 lost. Base: 12 matches. The 11 empty-stdout rows are decimals in `U+10F800..=U+10FFFF` (and the activity, state, and note consumers of those scalars). The two diffs are the escape-prefix rows below.

## What Java does with U+10F7FF

`&#1112063;` is `U+10F7FF`. `Character.toChars` accepts it. The us-ascii SVG writer emits it as a character reference, and the decoded text node contains that scalar. `cn6_escape_scalar` is `Edge U+10F7FF mark` at textLength 91.8818, size 14, weight 700.

`cn6_packed` is `&#1112062;&#1112063;&#1112064;` (`U+10F7FE`, `U+10F7FF`, `U+10F800`). Java keeps all three adjacent scalars. The text node is `Pack U+10F7FE U+10F7FF U+10F800 end` at textLength 100.5293.

The merge base stores a raw `U+10F7FF` and the exporter treats that scalar as an escape prefix of the next character, so the glyph disappears. `cn6_escape_scalar` becomes `Edge  mark` at textLength 81.3818 (10.5px less, one Lucida `.notdef`). `cn6_packed` drops `U+10F7FF` and keeps `U+10F7FE` plus `U+10F800` at textLength 90.0293. The tip prefixes a reserved decoded scalar with `ESCAPE`, so both rows match Java.

Decimals below `U+10F7FF`, 4- and 5-hex `<U+…>`, and the six-hex literals `<U+10F800>`, `<U+10FFFD>`, and `<U+10FFFF>` match on both trees. A six-hex form stays literal because the notation takes 4 or 5 hex digits.

## Residue

No held-out valid case fails on the tip. No addendum and no production change. No new target.

Standing residue, already out of lane and not exercised as a new CS miss: the class-colour parse of `#` inside `&#N;`; the `U+00C6` / `U+0080` / `U+2028` / `U+2029` advances; AtomText skipped in timing, gantt, salt, JSON, YAML, and nwdiag; a present local PNG drawn as `(Cannot decode)`; the T14.20 crash pages (`&#36;`, `&#92;`, surrogates, `<U+0000>`).
