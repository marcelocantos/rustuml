# T14.40 measurement

Recorded after the production draft, against base `cb550c0d67cc729ce764895005b7e86310c9f313`. Java revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`.

## What landed

`push_literal` restores hidden characters, then `manageAmpDiese` (`StringUtils.java` 472–484), then `manageUnicodeNotationUplus` (`StringUtils.java` 457–470), then `escape_creole_text`. A throw-equivalent match returns the restored string. The universal `&#N;` text-node encoder in the original account did not land; see `t14-40-addendum.md`.

## Cohort, tip versus base

| Cohort | Base | Tip |
|---|---|---|
| `golden_no_oracle` | 11,181/11,251, 70 failure names | same names, test passed |
| `golden_pairs` | 12,550 total, 10,497 passed, 754 failed, 1,299 skipped; panics 0, xml diff 754, other 0; golden_error skips 1,199 | identical |
| `parity_perturbations` failures | 2,727 | 2,717 |
| new perturbation failures | | 0 |
| `pipe_mode` | | 17/17 |
| `doc_claims` | | 2/2 |

Lost matches, lost rejections, new valid-input errors, and panics are zero. The no-oracle failure-name sets are equal, so the baseline was not rewritten.

Perturbation gains that are this mechanism:

- `tim-builtin-value-boundaries/checker-claude-round7/c7r_seq_uplus.puml` (`<U+000D>`)
- `c7r_class_amp13.puml`, `c7r_note_amp13.puml`, `c7r_title_amp13.puml` (`&#13;`)

CR is still owned by `svg_export`, which rewrites a raw carriage return to `&#13;`.

## Residuals

- Non-ASCII text nodes stay raw UTF-8. Java's SVG writer emits decimal `&#N;`. Strict comparison treats those as the same text. Whole-CLI bytes do not.
- `&#36;` and `&#92;` abort the atom and keep the literal. Java's unquoted `appendReplacement` throws. No crash SVG is invented (T14.20).
- `<U+D800>` and `<U+110000>` abort the same way.
- `<U+0000>` converts to NUL. The declared invalid probe's stdout contains a raw NUL byte and is not well-formed XML. That export crash is T14.20.
- `<U+E000>` converts. It is a declared invalid probe, not a positive case.
- Six-digit `<U+10FFFD>` stays literal.
