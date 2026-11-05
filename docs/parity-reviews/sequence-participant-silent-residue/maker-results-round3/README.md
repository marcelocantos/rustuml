# Sequence participant silent residue, round 3: maker results

Maker: Claude Opus 5.5. This is a maker record, not a review; an independent checker must judge it.

- **Account of record:** `account.json` here (commit `9b2e4758f`, before the drafts).
- **Production commits:**
  - `cde7b092c` (`UrlBuilder#getUrl`)
  - `b461817d4` (`addWithOrder`)
  - `cd82c28dd` (clippy only)
- **Shared evidence:** copied byte-identically from `../../sprite-command-post-tim-ownership/maker-results-round3/`. The SHA256 of each file is in `evidence-files.json`; `shared-summary.json` is that directory's `summary.json`.

## Obligation 4: `UrlBuilder#getUrl`

`parse/mod.rs::url_builder_url` follows `CommandParticipant#executeArg` and `UrlBuilder#getUrl` with `UrlMode.STRICT` (`Matcher#matches`).
- The whole `[[...]]` link is tried against QUOTED, ONLY_TOOLTIP, ONLY_TOOLTIP_AND_LABEL, LINK_TOOLTIP_NOLABEL, then LINK_WITH_OPTIONAL_TOOLTIP_WITH_OPTIONAL_LABEL.
- `Url#<init>` strips surrounding quotes from the URL. Its tooltip defaults to the URL.
- The tooltip is untrimmed and may be empty.

| Link | Java title | Rust title after |
|---|---|---|
| `[[u ring{x}]]` | the URL (label `ring{x}`) | the URL (the round-2 lost match is restored) |
| `[[u{ rings }]]` | ` rings ` | ` rings ` |
| `[[u{}]]` | empty | empty |
| `[[u{a{b}c}]]` | `a{b}c` | `a{b}c` |

## Obligation 5: `addWithOrder`

The sequence parser keeps Java's `participantsList`:
- `SequenceDiagram#addWithOrder` inserts before the first strictly greater `Participant#getOrder`.
- The order is 0 for implicit or unordered participants; an explicit order may be signed.
- `#putParticipantInLast` removes the participant and re-adds it.

The warning fires only when the position or the box changes. The three AT false warnings are gone. The remaining warnings (AS 2, 28 8) are all on inputs Rust mismatches.

## Results

Counts are in `summary.json`. Every cohort has 0 lost matches, 0 lost rejections, 0 new valid-input errors and 0 panics.

AT participant cases (10):
- 5 match before and after.
- 4 gains, including `ck_tooltip_label_brace_legacy`.
- 1 remaining mismatch, the teoz anchor: teoz participant anchors are not emitted, and Rust warns.

## Residue

- `topurl` is not applied.
- ONLY_TOOLTIP links keep the no-anchor model.
- `Url#getLabel` is not modelled.
- Class diagrams keep their existing tooltip reader.

## Pending relay request

Batch mk3 (`pending-request/`) includes 10 tooltip shapes and 5 re-declaration orders.
