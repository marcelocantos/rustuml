# TIM line continuation (🎯T14.22) round 5: independent checker review (Opus)

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/opus-tim-round5`. It reviews maker tip `36843c258`:
- Grok account `10c9fe37b` and draft `790726291`;
- merged base `9dc1af5f9`;
- addenda `5f74f24fa` and `6e2034c6f`;
- code `29c930a5f` and `c4b5a8436`;
- pre-registration `f9a6a6086`;
- lock `8bdfff0e3`.

The parents are round-4 code `9ed52f435` and parity-recovery `32684f81e`. The record is `../review-5.json`. Java was never run locally.

**Verdict: rejected. The packet is not accept-ready for local merge.**
- The round-5 code loses nothing against the merged base or `9ed52f435` in any of 19 cohorts (18,402 cases), held-out DD and DF included. All four obligations are accepted.
- Held-out DF has one Java-valid source, `cr5o_scope_uml_backslash_start` (`\startuml` … `\enduml`), that `32684f81e` renders and the tip turns into a valid-input error. No round-4-approved mechanism covers it.
- **Attribution:** the sprite/participant line, not round-5 code. The tip, `9dc1af5f9`, `9ed52f435` and `d28dd64a9` give the same error, "no registered sequence command accepts `\startuml`"; `cb550c0d6` and `32684f81e` render.
  - The mechanism: the sprite line's strict sequence admission meets a start-line test that accepts only `@start`.
  - Java's `StartUtils#isStartDirective` / `DiagramType#findStartTypes` accept `@` or `\` as the start character.
- **To clear it,** either:
  - port `\start` recognition for the main block (proposed target a); or
  - the owner explicitly adds this case to the approved set.

## Relays

| Batch | Directory | `reference-results.json` | Cases | Pre-registered |
|---|---|---|---|---|
| DD (maker or5) | `rustuml-relay-parent/opus-tim-r5-dd/` | `c4e76674…` | 13 | `f9a6a6086` (11:43) before results (12:42); 26 Rust outputs re-render identically |
| DF (checker CR5O) | `rustuml-relay-parent/opus-tim-r5-checker-heldout/` | `ffaaf255…` | 72 (29 resources) | `c9e6dda12`, before the relay; 144 outputs at the tip and the base re-render identically |

- **CR5O inputs:** `pending-request-cr5o/` holds the manifest (`2248a044…`), the bundle (`d8f72b01…`) and the predictions (`4a86cd2d…`). `request-df/SHA256SUMS` holds the relay hashes.
- **Held-out check:** no CR5O file (by sha256 or by normalised text) appears in any of the 29,920 files in the goldens, the perturbations, the relay-parent bundles or the round-4/round-5 caches.
- **DF validity:** 57 Java-valid and 15 invalid. `salt_theme` is an exit-0 crash page, which counts as invalid. My predictions were wrong on 6 cases (see `review-5.json`).

## Three-way comparison (T14.63 welcome-match overlay; `results/three-way-table.json`)

"Worse" means a lost match, lost rejection, lost Welcome match, new valid-input error, panic or timeout. Each cell gives worse/other changes.

| Cohort | n | tip match | Welcome match | vs `9dc1af5f9` | vs `9ed52f435` | vs `32684f81e` |
|---|---:|---:|---:|---|---|---|
| DF (held out) | 72 | 43 | 0 | 0/4 | 0/11 | **2**/26 |
| DD | 13 | 10 | 0 | 0/2 | 0/9 | 0/9 |
| CC | 74 | 18 | 8 (of 9) | 0/0 | 0/4 | 9/19 |
| CB | 39 | 24 | 6 | 0/0 | 0/0 | 0/36 |
| CA | 88 | 49 | 16 (of 19) | 0/0 | 0/0 | 3/70 |
| BZ, BY, BW, BU | 139 | 86 | 10 | 0/0 | 0/1 | 2/103 |
| BD, BA, AY, AZ, AX, AV, AT | 338 | 185 | 0 | 0/0 | 0/2 | 8/60 |
| 915 (AS, AQ, 790, 28) | 915 | 249 | 0 | 0/0 | 0/0 | 2/334 |
| retained | 4,174 | 2,742 | 0 | 0/0 | 0/1 | 13/331 |
| golden | 12,550 | 11,182 | 17 | 0/0 | 0/0 | 0/126 |

**Against `9dc1af5f9`:** SVG bytes change only in DD (3) and DF (6), and every change is a gain or a valid-input error becoming a mismatch.

**Against `32684f81e`: 39 worse** (`merge/worse-vs-32684f81e.json`).
- **37** are exactly the round-4 checker's list against `cb550c0d6`. The other 5 cases on that list are now fixed: the four Unicode headers and `tc1`.
- The maker counted 36, leaving out `cr4_cnt_exit_alone`, a lost Welcome match that `review-4.json` names. The sets are otherwise identical.
- **New from DF:**
  - `cr5o_scope_uml_usecase_theme`: `(Fly) as F` is rejected by sequence admission, as in the approved usecase cases `(Login)` and `Clerk --> (Pay)`. It fails without the theme too. **Inside** the approved usecase strict-admission set.
  - `cr5o_scope_uml_backslash_start`: **outside** the approved set. Inherited from the sprite line.

**Panics:** only in the unchanged `--yaml` set (retained 63, golden 148). **Timeouts:** 0.

**Rejection contract:** 0 introduced losses in 17 cohorts against all three trees.

## Obligations

1. **Inner comments on every line: accepted.**
   - All 15 DF comment probes on TIM bodies, sequence lines, includes, subs and themes match at every build.
   - EBNF: Java's `TLineType` COMMENT_SIMPLE (`^\s*'`) has no diagram-type test, so parity-recovery's EBNF short-comment exemption is wrong. `cmt_ebnf_short_line` is a Java error that every build accepts. `cmt_ebnf_short_indented` is Java-valid and a valid-input error at every build.
2. **Join only for `@startuml`, in result-list order: accepted.**
   - Java code: `PSystemBuilder.java:210-212`, `DiagramType#getTypes` (case `u`), `UmlSource#loadInternal`.
   - DD mindmap: Java does not join. It renders because `CommandMindMapOrgmode` takes `[*#]+`, so `#EEEEFF` becomes node `EEEEFF` before `Totara`.
   - The tip does not join either, but places `EEEEFF` after `Totara` (`theme_tail` relocation). That is a mismatch, not an error, and it predates the packet.
   - DD wbs: a Java error that every build accepts.
3. **Unicode 15.0 table: accepted.**
   - My own generator over UCD 15.0.0 reproduces the table exactly: 48,965 letters in 380 runs, and Nd 370.
   - Unicode 17 differs by exactly the 16 code points the maker named.
   - All 15 DF header probes classify as Java does at the tip.
4. **Lock: accepted.**
   - `edge_pragma_useVerticalIf` is byte-identical to Java except the root height (135 vs 134 px), which the frozen comparator accepts.
   - The 69 no-oracle failure names are identical at the tip, the base and `9ed52f435`.
   - README, STABILITY, the help text and `doc_claims` all carry 11,182/11,251.

## Gates (`gates/`)

| Gate | Result |
|---|---|
| Workspace lib | 1,668 passed, 0 failed |
| `pipe_mode` | 17/17 |
| `doc_claims` | 2/2 |
| `golden_no_oracle` | Pass. Tip 11,182/11,251; base the same, failing only on the unlocked 759→760; `32684f81e` 11,181 |
| `golden_pairs` | 10,498/753/1,299 at the tip and the base, with identical per-directory failures |
| `parity_perturbations` | 3,835 failing pairs at the tip and the base, with identical names. On the 6,379 sources shared with `32684f81e`, 0 fail at the tip that pass at `32684f81e` |
| `harness_guards` | 5 pass. `decimal_literal_ratchet` and `parity_review_records_are_complete` fail identically at the tip and the base |
| clippy `-D warnings` | `rustuml-layout` and `rustuml-math` pass. `rustuml`, `rustuml-parser`, `rustuml-render` and `rustuml-oracle` fail on the unused `looks_like_description_bracket_command`, which fails identically at `32684f81e` and `9ed52f435`. The `--all-targets` lint census is identical at all three trees |

## Process and frozen items

- Each account or addendum precedes its code: `10c9fe37b` → `790726291`, `5f74f24fa` → `29c930a5f`, `6e2034c6f` → `c4b5a8436`. The account and addendum commits touch only docs.
- Round-5 non-test code contains no fixture-keyed literal.
- Against `32684f81e`, `crates/rustuml-oracle`, `scripts`, `.gitmodules` and the non-perturbation `test-diagrams` differ only in the `doc_claims` and baseline pins.
- Every frozen item is unchanged: `GEOM_EPS`, markers, the park list, the error census, the allow-lists, the strict-XML list and the 15-source approved valid-error set.

## Files

- `review-5.json`
- `results/`:
  - `three-way-table.json` and `summary.json.gz`;
  - `df-percase.json` and `dd-percase.json`;
  - `verify-fresh.json`, `ucd-check.txt` and `binaries.sha256`.
- `merge/worse-vs-32684f81e.json`
- `rejection/`
- `gates/`
- `manifests/`
- `pending-request-cr5o/` and `request-df/`
- `support/`. `support/unchanged-checker-scripts.sha256` records the unchanged `replay.py`, `classify.py`, `rejection_contract.py` and `prepare.py`.
