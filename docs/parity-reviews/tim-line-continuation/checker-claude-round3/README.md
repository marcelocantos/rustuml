# TIM line continuation (🎯T14.22) round 3: independent checker pre-registration

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/tim-continuation-round3-claude`. This round reviews maker tip `f2accc662`, which has code `d28dd64a9` and merged base `a5720e095`.

This commit is the pre-registration only. It was written before any Java result for batch CR3 existed. Java was never run locally.

## Batch CR3 (held out)

The batch has 88 cases, 78 of them predicted Java-valid, and 22 include resources. It is staged immutably at `~/Library/Caches/rustuml-tim-continuation-round3-claude-checker/relay-cr3/`, and `pending-request-cr3/` holds a copy.

| File | sha256 |
|---|---|
| `reference-manifest.json` | `443cea19516ac0801b6eb6136304a7480f13a0e67551cb9b06131b962d0ba1c4` |
| `reference-input-bundle.json.gz` | `8baa2d7f502dbee6689d91884bc1b04b6250fbc2520d1e6380e04a10a8fe5737` |
| `cr3-rust-predictions.json` (Rust at `d28dd64a9`, `a5720e095`, `93bf53e18`, `9834a7194`) | `6e88c3003f2b24087b5a94297a0107be25354acfc3801de209becae8e5891899` |
| `cases-with-java-predictions.json` | `2b7d4e57f570da5dc9def26a7de86ab73620d344edf3141aa1e4db4e4d3826a5` |

## Axes

- **A. Typed TIM lines (34 cases).** Each of 11 directive kinds sits at the start, middle and end of the block, around a post-TIM tail (`%chr(92)`). The kinds are `!define`, `!procedure`, `!if`/`!endif`, `!include`, `!theme`, `!option`, `!pragma`, `!assume`, `!log`, `!dump_memory` and `!startsub`. One extra case uses a teoz pragma.
  - Java's `TLineType` classes `!pragma` and `!assume` as PLAIN. They are lines of the TIM result list, so they count toward `getTotalLineCount`, and the tail joins them.
  - Every other kind yields no entry.
- **B. The two-line rule at its edges (14 cases).**
  - Count 1: the end line is continued.
  - Count 2: two content lines and a continued end, comments only, directives only, two tails, activity text.
  - Count 3: a blank line, or a line before or after the tail.
  - Two lines without a join: `!option handwritten`, `!pragma`.
  - A mindmap control.
- **C. `\end…` forms (20 cases).**
  - Bare `\end`; `\endless`, indented; `\endmindmap` inside `@startuml`; trailing text; case; a double backslash.
  - An end line inside a block comment, an inactive `!if`, a procedure definition or a note body.
  - U+001F before the end (Java whitespace), and U+0085 and NBSP before it (not Java whitespace).
  - A TIM-generated `@enduml` or `\enduml` (the `PSystemCommandFactory` end peek).
  - A mindmap block, a second diagram in the Pipe text, and no `@end` line at all.
- **D. Wrapped and raw includes (20 cases).**
  - Multiple blocks, with `!N` and `!id` suffixes; Java ignores them for local files (TContext.java:798–846).
  - A first block of another family.
  - Headers: `##`, `<x>`, U+00B2 (not `isLetterOrDigit`).
  - A BOM at the start, a BOM before a header, a BOM on a body line, and a raw include with a BOM. `ReadLineReader` strips a leading U+FEFF from every line.
  - CRLF with comment-skipping continuation.
  - A tail that runs into a second block, nested wrapped files, definitions, a `\enduml` or trailing-text end, and `\startuml`.
  - A mindmap family include.
  - A raw include holding `@enduml`.

The Java predictions are in `cases-with-java-predictions.json` (`java_prediction`). Validity will be decided from the raw Java SVG and stderr. Welcome pages will be classified explicitly.
