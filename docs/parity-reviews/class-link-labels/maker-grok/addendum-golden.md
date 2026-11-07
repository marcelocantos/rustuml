# Class link labels — golden measurement before the baseline lock

Written after the xlabel port was measured and before
`test-diagrams/no_oracle_baseline.txt` moves. This is not a verdict.

## What was measured

Official `compare_svg_strict` (`GEOM_EPS` 0.02, unchanged) of the tip
binary against relay batch `shared-svg-round9-c9a`:

- Acceptance cases all MATCH: `c9a_af_multiline_vd`,
  `c9a_af_multiline3_size16`, `c9a_af_style_bold`,
  `c9a_af_style_italic`, `c9a_af_stereo_skinparam`,
  `c9a_af_stereo_global_vd`, `c9a_af_roles_size16`, `c9a_af_size9`,
  `c9a_af_ortho_vd`, `c9a_af_cn_italic_mark`,
  `c9a_af_cn_italic_mark_ctrl`. Base is DIFF on every one of them.
- The other class `c9a_af_*` cases also MATCH. The four non-class
  cases (`component`, `deploy`, `object`, `usecase`) are byte-identical
  to base. `c9a_af_state_vd` MATCHES and is byte-identical to base.
- Recorded worker cwd
  `/Users/marcelo/rustuml-reference/scratch/worker.KKJX2HOy/new` was
  absent. The sources have no includes, so the CLI was run from `/tmp`.

No-oracle product tier, same corpus of 12,550 goldens, release test
`golden_no_oracle`, base `cb550c0d6` versus this tip:

- Base 11,181/11,251, exit 0, matching the checked-in baseline.
- Tip 11,182/11,251. The only failure-name change is a gain:
  `rendering/splines/splines_multilevel_inheritance`. Zero losses.
- That golden is a class diagram (`skinparam linetype ortho`,
  inheritance, and `Dog --> Cat : chases`). The gain is the ortho
  `xlabel` placement from the previous addendum, not a new constant.
- Family `class` stays 1,963/1,971. Family `rendering` is 95 → 96
  eligible 98. Every other family is unchanged.

## What the following commit locks

`rendering 95 98` becomes `rendering 96 98`. The product total in
README, STABILITY, `--help-agent`, and `doc_claims` moves from
11,181/11,251 to 11,182/11,251. 11,182/11,251 is still 99.4% at one
decimal, so the percentage text stays. No other baseline moves.
