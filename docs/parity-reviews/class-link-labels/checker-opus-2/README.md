# Class link labels: checker re-review 2 (Opus)

This is an independent checker review of maker tip `da6c18866`. The code under review is
`50cec1dd1` and `50218c805`, compared against merge base `9fb2e52a6`, with goldens at
`9efdc12f`. The record is `../review-2.json`.

**Verdict: rejected. Not accept-ready for local merge.**

## Blocking

1. **Clippy is worse than at the merge base**, which the brief does not allow. Commit
   `50cec1dd1` adds 4 warnings:
   - `rustuml-parser` goes from 1 to 4 warnings. `protect_creole_links` and
     `restore_creole_links` (`parse/class.rs:1977`, `:1990`) raise `manual_repeat_n` twice
     and `manual_str_repeat` once.
   - `rustuml-render` goes from 20 to 21 warnings. The `Placed.rows` tuple
     (`render/class.rs:13067`) raises `type_complexity`.

   See `clippy/`.
2. **`er/er_ecommerce_schema` gets worse under `compare_svg_strict`** on the product
   (CLI) path, going from DIFF 99 to DIFF 100. Java's canvas is 1035 wide, base's is 1036
   (within tolerance) and the tip's is 1105.

   The cause is the label `parent of` on the self-loop `Category ||--o{ Category`. Sending
   it as `xlabel` is a faithful port of `SvekEdge.appendLine` (lines 425–431). The problem
   is Rust's pre-existing self-loop route: it is a Bezier loop on the right of `Category`,
   while Java routes the loop orthogonally on the left. The label lands at x 1027.19.
   Java puts it at 753.38, and base put it at 936.53.

   The fix belongs in the route, not in a self-loop special case. The parent can instead
   sign off on this one golden.

There are no held-out strict losses, no fitted constants and no coordinate surgery.

## Counts (strict = vendored `compare_svg_strict`, Java exit 0)

| Batch | Base | Tip | Lost |
|---|---:|---:|---:|
| DA (held-out, maker unseen) | 3/55 | 25/55 | 0 |
| CO | 13/94 | 75/94 | 0 |
| CK | 1/13 | 13/13 | 0 |
| CT | 0/46 | 45/46 | 0 |
| CY | 0/4 | 4/4 | 0 |
| c9a | 100/288 | 113/288 | 0 |
| c11a | 245/515 | 251/515 | 0 |

- **`golden_no_oracle` failures:** 70 at base, 69 at the tip. The one change is the gain
  `rendering/splines/splines_multilevel_inheritance`.
- **`golden_pairs`:** identical on both trees (10,497 pass, 754 fail, same names).
- **`parity_perturbations` failures:** 2,713 at base, 2,694 at the tip (19 gains, 0 losses).
  After this review adds 29 `class-link-labels` pairs, all 29 pass at the tip and 27 fail at
  base.

## DA registration

The predictions in `rust-predictions.json` (`f3e7fa96…`) were written at 10:32:26. The
Java results (`08cee6fb…`) arrived at 10:35:03. I rebuilt both commits and re-rendered all
56 cases. All 112 renders match the registered exit code, stdout and stderr byte for byte.
The binary hashes differ from the registered ones because the build path is embedded in
the binary.

## Non-blocking findings (detail in `review-2.json`)

- **`relationship_link_parts` is not `UrlBuilder`.**
  - In `url_quoted_bar`, the href keeps its quotes.
  - In `url_brace_nl`, `{extra}` inside the label is taken as a tooltip.
  - Single-line `[[url|x]]` still goes through `creole.rs`, whose bar split is
    pre-existing. So the claim "`|` is drawn as Java draws it" holds only for multi-line
    labels.
  - `url_bar_*` and `url_brace_in_label` rise in strict count because the packet's
    Java-correct parser masking now hands those URLs to that pre-existing parser.
- **`TextBlockArrow2` should use integer `triSize / 2`.** The port takes `triSize / 2.0`.
  This is the cause of CO `g4k_magic_sz7` and `g4k_magic_sz9` (DIFF 1 each).
- **Ortho TB xlabels inside nested packages sit 43 px left of Java** (pre-existing). This
  is the remaining DIFF 3 in DA `ortho_*_tb`.
- **Other pre-existing residues:**
  - `#--`, `x--` and `+--` links are dropped.
  - A trailing backslash does not join `@enduml` (`g4r2_trail_bs`).
  - `<code>` in a label is rendered as Creole instead of being drawn literally.

## Files

| File | Contents |
|---|---|
| `scripts/` | Render, score, sweep and materialise helpers, plus the scorer crate (`compare.rs` vendored verbatim; it is unchanged from base to tip). |
| `da-render.json`, `da-score.json` | DA hashes and per-case verdicts for base and tip. |
| `cohort-*-score.json` | Per-case verdicts for CO, CK, CT, CY, c9a and c11a. |
| `golden-sweep-changed.json` | The 7 goldens whose CLI bytes differ between base and tip. |
| `no_oracle_names-*.txt`, `pairs_names-*.txt`, `pert-names-*.txt` | Gate failure names, base and tip. |
| `guards-*.txt` | `harness_guards` review-record messages, normalised. |
| `clippy/` | Per-crate clippy diagnostics with line numbers stripped. |

Java evidence (immutable) lives in `~/Library/Caches/rustuml-relay-parent/` under
`grok-g4-recheck-heldout` (DA), `grok-g4-checker-heldout` (CO), `grok-g4-class-link`
(CK), `grok-g4-ct` (CT), `grok-g4-cy` (CY), `shared-svg-round9-c9a` and
`shared-svg-round11-c11a`.
