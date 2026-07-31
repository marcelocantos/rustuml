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

For every row, the cited commit contains byte-identical current account
content, and Git records the account commit as an ancestor of the cited
implementation commit.

## Unresolved

These records remain accepted as authored but cannot select positive evidence
under the stricter schema. No replacement claim was invented.

| Mechanism | Blocking evidence gap |
| --- | --- |
| `deployment-opale-peer-svek-node-gate` | The current canonical account is at `9aeb64ed5f58b109ecbb19178359c2b4a9bfda4b`; the review-attested implementation `4a9bccfd78b36af225e6a194587535e9a744c939` does not descend from it. |
| `description-link-note-color-factory-consumption` | The current canonical account is at `1ee8a31fb069a02e566d797c9f4d21d2ce101035`; the preserved review does not attest a distinct implementation revision that descends from it. |
| `tim-procedure-following-source-origins` | The current canonical account is at `d04b7ec69d7f86aeafbc9202da3c41ef7569ddac`; its accepted review lacks both a distinct attested implementation descendant and explicit checker tool and independence claims. |
| `state-skinparam-canonical-key-resolution` | Its recorded account and implementation commits are usable, but the accepted review lacks explicit checker tool and independence claims. |
