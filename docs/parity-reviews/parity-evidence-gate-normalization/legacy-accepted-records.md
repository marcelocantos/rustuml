# Legacy Accepted Record Migration

This migration preserves the authored `ACCEPT` or `ACCEPTED` verdicts. It adds
only canonical aliases whose values were already recorded or whose current
`account.json` bytes were verified against Git history.

## Migrated

| Mechanism | Account commit | Implementation commit |
| --- | --- | --- |
| `state-font-family-legacy-resolution` | `0e1e7a90214f2604a68da2155ee309ca8aa5ba47` | `976f8bf7780f6846dc0b9a76b0728f6a67f4ee29` |
| `ordinary-note-command-end-grammar` | `04e70e40dad8f344aedebba317eedfb8b259f3fc` | `15980239c9971d79752cc3914b844f7376d167c2` |
| `class-attached-note-rankdir-position` | `85513d6429cf7dfe88e554b31737bd37e79ac704` | `bf4b9c0a924b5dfe10c0e85f69a39e8bf6d3467d` |
| `class-note-opale-spline-eligibility` | `e9bec60554110bbc04c4ead97e9f08a289a3ab2f` | `bf4b9c0a924b5dfe10c0e85f69a39e8bf6d3467d` |
| `component-attached-note-contextual-identity` | `948ac3ac4419fdb080a1622c989dca0f879c634a` | `8e3293558e45963ebd57d910370fbfa8d3660da3` |
| `class-magma-semantic-leaf-membership` | `27ce0ddafda82e1d45f662f09d21f4b11535fa22` | `03eb672a5c186c99fc0464cd2af1954af2944861` |
| `display-raw-newline-tokenization` | `b93fbb83e7b34f5f2be228824de415332d476a15` | `b9b5e3805458481c8e595247a18229c511aa52c3` |
| `class-package-direct-standalone-layout` | `4fe430091e56f22963be85e0592e7b65e0511829` | `1482ade4ff70df8c2da81848b11956d66179a029` |
| `class-quoted-symbol-container-dispatch` | `6ffe53bbd0b1fcae5eade40d41577af48f7555da` | `ee0f6d52c547f980c5db65b3447c07c9ac792b14` |
| `class-symbol-container-alias-parse` | `f75977eaf589e7bd40f5308070f0638a1811590b` | `1c80b8b0e691700fe5fcaac9ae0fa5f7b9ccefd9` |
| `command-factory-note-color-resolution` | `499e0c0225138a5b8ef5a901dc220bd60475f834` | `6256529d1b53cbfb3ade2351ee15cfedaf21c531` |

For every row, the cited commit contains byte-identical current account
content, and Git records the account commit as an ancestor of the cited
implementation commit. The quoted-container review's string-only
`fresh_inputs` alias was redundant with its object-shaped
`fresh_perturbations`; the alias was removed without losing any source,
golden, axis, or result fact.

## Preserved Rejections

The rejected canonical review records listed below were moved, without editing
their bytes or verdicts, to `review-rejected-<sha256-prefix>.json`. They are
archival counterevidence and remain auditable, but are not eligible to select
positive evidence under the strict schema.

| Mechanism | Archived review records |
| --- | --- |
| `stack-transparent-inset-limitfinder-frontier` | `review.json@597ee2b5`, `review-2.json@dfc8cb0f` |
| `svg-logical-monospace-serialization` | `review.json@3232c7d7`, `review-2.json@639dd02a` |
| `class-quoted-symbol-container-dispatch` | `review.json@4467575c`, `review-followup.json@69d166ec`, `review-followup-2.json@70faf074`, `review-followup-3.json@16ce5244` |
| `class-symbol-container-alias-parse` | `review.json@87463ba9` |
| `class-together-structural-layout` | `review.json@57cd306f` |
| `command-factory-note-color-resolution` | `review.json@4d246925`, `review-followup.json@e413e0c2` |
| `deployment-hidden-link-lifecycle` | `review.json@7be26cb8` |
| `preprocessed-source-comment-identity` | `review.json@0b207f61`, `review-2.json@418e3756` |
| `state-autonomous-dot-option-spacing` | `review.json@3aaab314` |
| `state-choice-polygon-serialization` | `review.json@a478a7b7` |
| `state-composite-entry-exit-frontier` | `review.json@b968263f`, `review-2.json@090de99d`, `review-3.json@fe6814b6` |
| `state-flat-svek-painted-envelope` | `review.json@fe9b5701` |

## Unresolved

These records remain accepted as authored but cannot select positive evidence
under the stricter schema. No replacement claim was invented.

| Mechanism | Blocking evidence gap |
| --- | --- |
| `deployment-opale-peer-svek-node-gate` | The current canonical account is at `9aeb64ed5f58b109ecbb19178359c2b4a9bfda4b`; the review-attested implementation `4a9bccfd78b36af225e6a194587535e9a744c939` does not descend from it. |
| `description-link-note-color-factory-consumption` | The current canonical account is at `1ee8a31fb069a02e566d797c9f4d21d2ce101035`; the preserved review does not attest a distinct implementation revision that descends from it. |
| `tim-procedure-following-source-origins` | The current canonical account is at `d04b7ec69d7f86aeafbc9202da3c41ef7569ddac`; its accepted review lacks both a distinct attested implementation descendant and explicit checker tool and independence claims. |
| `state-skinparam-canonical-key-resolution` | Its recorded account and implementation commits are usable, but the accepted review lacks explicit checker tool and independence claims. |
| `class-together-structural-layout` | The accepted `review-2.json` names only a task id and model. It records neither checker tool nor independence, and has no revisions object. |
| `deployment-hidden-link-lifecycle` | The checker and Git provenance can be normalized, but its valid held-outs record observations rather than a pass result accepted by the strict selector; no full no-oracle comparison claim is present to promote. |
| `preprocessed-source-comment-identity` | The review names implementation `be4f0c2feb131cfde8f46b239ffefd4a3e46a860`, but no byte-identical current account commit precedes it; its checker object also lacks explicit model and tool fields. |
| `state-autonomous-dot-option-spacing` | The accepted `review-2.json` names only a task id and model and has no attested account, implementation, or Java revisions. |
| `state-autonomous-root-painted-normalization` | There is no directory-local canonical `account.json`; the accepted review is therefore not eligible to select evidence. Its `checker_identity` also cannot satisfy the required single explicit reviewer/checker container. |
| `state-choice-polygon-serialization` | The accepted `review-2.json` names only a task id and model and has no attested account, implementation, or Java revisions. |
| `state-composite-entry-exit-frontier` | The accepted `review-4.json` names only a task id and model and has no attested account, implementation, or Java revisions. |
| `state-flat-svek-painted-envelope` | The accepted `review-2.json` names only a task id and model and has no attested account, implementation, or Java revisions. |
