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
| `stack-transparent-rectangle-stroke-state` | `review.json@51451c51` |
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

## Archived Legacy Accepts

The records below retain their authored accepted verdict and every original
byte under `legacy-accepted-<sha256-prefix>.json`. That filename is
deliberately nonselecting: it preserves the historical claim without allowing
the stronger gate to treat incomplete provenance as current approval. Each
mechanism remains represented by its canonical `account.json`; a new
independent review is required before it can select positive evidence.

| Mechanism | Archive | SHA-256 | Blocking evidence gap |
| --- | --- | --- | --- |
| `class-together-structural-layout` | `legacy-accepted-b19d87fb.json` | `b19d87fb2da5d22e34baba96e537181782c2e80ad998d01e9c2cad56db1c6596` | No checker tool or independence attestation and no revisions object. |
| `deployment-hidden-link-lifecycle` | `legacy-accepted-9076c079.json` | `9076c079783060d78e72b3d76d0e3c126958631a26c7919e705ba6a8a3830594` | Revision aliases are incomplete, evidence path aliases conflict, and the valid held-outs record observations rather than a strict no-oracle pass with two positive axes. |
| `deployment-opale-peer-svek-node-gate` | `legacy-accepted-3bce3bb4.json` | `3bce3bb4c81d924b376c4ba7fca5766d520308796de0d058f0709a975b296c38` | The reviewer does not separately attest its tool, and the attested implementation does not descend from the current canonical account. |
| `description-link-note-color-factory-consumption` | `legacy-accepted-e36666b4.json` | `e36666b47236a40d42d882cae10c4bc50fa802fb11ca944b43758fe67d37c88b` | No distinct implementation revision descending from the current canonical account is attested. |
| `preprocessed-source-comment-identity` | `legacy-accepted-d5411865.json` | `d541186500881e0ed6cf9ff74ca63fc8572169d3bef25e7b76887b4bacbcdac0` | No byte-identical canonical account commit precedes the named implementation, and the checker object lacks explicit model and tool fields. |
| `state-autonomous-dot-option-spacing` | `legacy-accepted-af5492cd.json` | `af5492cdd963484534501fee72825784fcb37265760d6223eea8244ef095e07d` | No checker tool or independence attestation and no account, implementation, or Java revision attestation. |
| `state-autonomous-painted-bounds` | `legacy-accepted-e2654044.json` | `e2654044268f1a9857be049cf8aaa2fa345c859b732b4d48703941b33d54ac8a` | The third-round checker records only a task id and model and has no revisions object; the two earlier rejected reviews remain canonical counterevidence. |
| `state-autonomous-root-painted-normalization` | `legacy-accepted-5150c843.json` | `5150c8439d76e3c5bc0205c424b948a005c28ec71a828d4b8ca9144941b9a59a` | The review uses a noncanonical checker identity container and incomplete revision aliases. The exact proposed account was copied into the canonical directory with SHA-256 `46b0227ec4ef007d6d6eb62e887e27d6e603fc943148813afcd4a5d11fffac3c`. |
| `state-choice-polygon-serialization` | `legacy-accepted-2a320ac8.json` | `2a320ac8374c99b1069660c0635fd160406290630e91dc22f6bdb05b6203c698` | No checker tool or independence attestation and no account, implementation, or Java revision attestation. |
| `state-composite-entry-exit-frontier` | `legacy-accepted-eff0f27e.json` | `eff0f27e04b2a2465904a58759e69018dde1029aaf4856634350007392b7ce01` | No checker tool or independence attestation and no account, implementation, or Java revision attestation. |
| `state-flat-svek-painted-envelope` | `legacy-accepted-79bb9205.json` | `79bb92057cd5506d74db1b2d928606ceafef640680f4fab30ab9974ec6c8dc4e` | No checker tool or independence attestation and no account, implementation, or Java revision attestation. |
| `state-skinparam-canonical-key-resolution` | `legacy-accepted-1730f87c.json` | `1730f87c01c9dd14b7fc8889fcbe7fdefee03e2de9bcc160e91bd4e47a034dbe` | Account and implementation commits are recoverable, but the original record does not explicitly attest checker tool or independence. |
| `tim-procedure-following-source-origins` | `legacy-accepted-3e1ed6fc.json` | `3e1ed6fc8cb1d8c22e64aff5c16da02a334c42a16ba80589e58414886d5d5085` | No distinct implementation descendant and no explicit checker tool or independence attestation. |
