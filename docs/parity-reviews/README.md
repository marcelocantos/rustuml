# T14 parity review records

Parity changes use a three-commit maker/checker protocol:

1. The maker commits `<mechanism>/account.json` before changing production
   code. This records the causal model hypothesis and planned perturbation
   axes.
2. The maker implements the model change and ratchets the no-oracle baseline,
   referencing the account commit.
3. A separate checker task, preferably using another model family, attempts
   to disprove the mechanism. The checker commits every generated `.puml`
   input and Java `.svg` output under
   `test-diagrams/perturbations/<mechanism>/`, then authors
   `<mechanism>/review.json`.

The maker must not write or edit `review.json`. A rejected review and every
counterexample it found stay in the record and executable perturbation suite;
a subsequent checker writes `review-2.json`, `review-3.json`, and so on after
corrective implementation commits. No parity change is accepted or merged
until its latest review is accepted and every perturbation, including those
from earlier rejected reviews, passes.

`account.json` has this shape:

```json
{
  "schema_version": 1,
  "mechanism": "state-cross-boundary-routing",
  "java_revision": "40-character Git commit",
  "java_locations": ["package.Class#method"],
  "rust_locations": ["crates/rustuml-render/src/state.rs::function"],
  "invariant": "The model property that both implementations must preserve.",
  "causal_divergence": "Why the current Rust model violates that property.",
  "planned_change": "The model/data-flow change, not expected SVG coordinates.",
  "perturbation_axes": ["topology", "label width"]
}
```

`review.json` and numbered successors have this shape:

```json
{
  "schema_version": 1,
  "mechanism": "state-cross-boundary-routing",
  "reviewer": {
    "task_id": "independent task or agent identifier",
    "model": "model/tool identifier"
  },
  "account_commit": "40-character Git commit",
  "implementation_commit": "40-character Git commit",
  "java_revision": "40-character Git commit",
  "commands": ["command used to generate Java SVGs", "cargo test command"],
  "perturbations": [
    {
      "source": "test-diagrams/perturbations/state-cross-boundary-routing/case.puml",
      "golden": "test-diagrams/perturbations/state-cross-boundary-routing/case.svg",
      "axes": ["topology", "label width"],
      "result": "pass"
    }
  ],
  "findings": ["What the checker tried and observed."],
  "verdict": "accepted"
}
```

The harness requires at least one review for every account, validates the
required fields, requires exact `.puml`/same-stem `.svg` fixture pairs, and
requires the latest numbered review to be accepted with all of its
perturbations passing. Earlier reviews may record `"result": "fail"` and
`"verdict": "rejected"`; their fixtures remain in the executable suite.

Git history is the audit trail proving that the account preceded
implementation and that the checker, not the maker, authored the review.
