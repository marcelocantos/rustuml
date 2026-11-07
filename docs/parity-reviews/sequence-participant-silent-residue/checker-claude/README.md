# Sequence participant silent residue: independent review, rejected

Reviewer: Claude Opus 5.5 (`claude-opus-5-5`). This uses the same builds, cohorts and gates as `../../sprite-command-post-tim-ownership/checker-claude/`, and the shared evidence is copied here byte-identically (the SHA256 of each copy is in `review.json` → `evidence_files`). Production: `ecbd75ec8` and `87bdb0feb`. Account: `65b5a1633`.

## Verdict

**Rejected.** One held-out Java-valid AT case loses its whole-output match: `ck_tooltip_label_brace_legacy`.
- In `[[https://example.org/ring ring{x}]]`, Java's `UrlBuilder#getUrl` (LINK_WITH_OPTIONAL_TOOLTIP_WITH_OPTIONAL_LABEL) reads `ring{x}` as the label, so the tooltip is null and the title stays the URL.
- `extract_link_tooltip` takes any `{…}` in the link, so Rust's title becomes `x`.

| Cohort (participant cases) | n | Before match / mismatch | After |
|---|---:|---|---|
| AT | 10 | 5/5 | 5/5 (1 lost, 1 gained: `ck_create_tooltip_legacy`) |
| AS | 13 | 1/12 | 2/11 |

## Further divergences (not losses; they mismatch before and after)

| Link | Java title | Rust title |
|---|---|---|
| `{ rings }` | ` rings ` (untrimmed) | `rings` |
| `{}` | empty | the URL |
| `{a{b}c}` | `a{b}c` (`LINK_TOOLTIP_NOLABEL`) | `a{b` |

## False warning

`ck_redeclare_order_stable_{legacy,teoz}` and `ck_redeclare_order_lower_legacy` match Java whole-output, but the packet warns that the participant "moves to last".
- `putParticipantInLast` re-adds with `addWithOrder`, which inserts before the first participant of strictly greater order.
- An explicit order therefore keeps its position, but the Rust key check assumes every order is the default.

## What holds

- The created head is drawn without its link (`ArrowAndParticipant#drawInternalU`).
- The `{tooltip}` title gains in 790 and 28 reproduce.
- The other new warnings fire only on mismatching inputs.
- The AS colour-free controls confirm that the remaining note and box residue is geometry, not colour.
- `url_tooltip` is an additive model field.

## Next obligations

1. Port the `UrlBuilder#getUrl` alternatives in order, in STRICT mode, with an untrimmed and possibly empty tooltip.
2. Model `addWithOrder` in the re-declaration warning.
