# Checker batch Q8: pre-registration (no Java result exists)

Held-out probes for the T14.22 round-7 re-check (maker tip `df078f7d7`, code `16b1c4dcf`; merged base `6d195c178`).

- Staged at `~/Library/Caches/rustuml-opus-tim-r7-recheck/q8/` (`reference-manifest.json`, `reference-input-bundle.json.gz`): 51 cases, 33 predicted Java-valid, 3 resources.
- `q8-rust-predictions.json`: exit, SVG sha256 and stderr at the tip `df078f7d7`, the merged base `6d195c178`, `9ed52f435` and `f6a738af1`, recorded before any Java result. Java validity predictions are each case's `intended_valid`; uncertain ones carry `unsure` in `axes`, and `java_lines N,M` gives the predicted `data-source-line` of the messages.
- `heldout-check-q8.json`: 0 hits among 301,502 corpus files (goldens, perturbations, `docs/`, every other `rustuml-*` cache, every relay bundle).
- Generator: `../support/gen_probes_q8.py` (its docstring names the Java mechanisms behind each prediction).
- Java was not run locally.
