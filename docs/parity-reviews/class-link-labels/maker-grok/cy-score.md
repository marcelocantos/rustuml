# Class link labels — CY score

Scored the four pipe probes, then re-measured the final tip against the
merge base. Validity comes from the Java exit code and stderr.
`compare_svg_strict` uses `GEOM_EPS` 0.02. This is a measurement, not a
checker review.

## Binaries

Tip commit `50218c805`. Tip binary
`b2008a28be42efbe34965aa26519a5daeac792d3a7a6b9d9add04867030f459a`.
Merge base `5c035c88fb07be7a023c7b4e3e8ec5d82bbf1e7bcf787f77b3b240a27fca30f6`.
Java results
`d25c877b24e25f52aa2792b147bf1fedfb5827e40759d8c227f33beb266b26d9`.
All four cases exit 0 with empty stderr.

## CY

| Case | Java | Tip | Merge |
|---|---|---|---|
| `g4p_pipe_neighbors` | valid 0 | 0 MATCH | 0 DIFF 15 |
| `g4p_pipe_two` | valid 0 | 0 MATCH | 0 DIFF 11 |
| `g4p_label_pipe` | valid 0 | 0 MATCH | 0 DIFF 11 |
| `g4p_pipe_inline` | valid 0 | 0 MATCH | 0 DIFF 12 |

Lost matches: 0. The tip text matches Java on every case. `g4p_label_pipe`
draws `x|y` and `z`. The other three keep the bar inside the URL and split
on the newline.

## Zero loss

A lost match is a Java-valid diagram that matches on the merge base and
fails on the tip. Identical stdout cannot be a loss. A lost rejection is
Java-invalid input that the merge base rejects and the tip accepts.

| Cohort | n | changed | lost | lost rejection | new error | tip panic |
|---|---:|---:|---:|---:|---:|---:|
| CO | 100 | 10 | 0 | 0 | 0 | 0 |
| CK | 13 | 0 | 0 | 0 | 0 | 0 |
| CT | 70 | 39 | 0 | 0 | 0 | 0 |
| c9a | 288 | 0 | 0 | 0 | 0 | 0 |
| c11a | 515 | 0 | 0 | 0 | 0 | 0 |
| golden | 12550 | 4 | 0 | 0 | 0 | 0 |
| perturbations | 6365 | 0 | 0 | 0 | 0 | 4 |
| retained `c9a_af_*` | 38 | | 0 | | 0 | 0 |

CT's 39 byte changes are 27 Java-valid diagrams and 12 Java-invalid ones.
None of the valid changes is a lost match. The 12 invalid changes are the
left-token syntax errors whose bytes already differed on the merge base.
That under-rejection stays 🎯T14.96.

The four perturbation panics (`dash_01`, `dash_03`, `dash_06`, `dash_07`)
exit 101 on both binaries with the same stdout, at `parse/class.rs:285`.
They are not this packet. The four golden byte changes are the same ER
schemas recorded in `fix-score.md`, and none lost a match.
