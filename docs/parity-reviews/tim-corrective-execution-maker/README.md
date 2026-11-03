# TIM corrective execution maker packet

This is implementation evidence, not an independent review or acceptance record.
The five corrective accounts were committed in
`819485f4001a94056a192e4e8688729d4c9de3e8` before production edits. They respond to
checker `a8c96282f4ed55cb638f3a3e4d6440ee5f1aebf7`; all ten preceding claims remain
rejected until a new independent checker evaluates the correction.

The implementation changes function execution kind, typed memory and argument
binding, eager operands and first-error propagation, raw external-sub selection,
and width-sensitive zone display names. Integer arithmetic follows TValue's
signed Java int behavior. Root's two parser error handoffs and approved raw-reader
origin assertion are preserved. Existing Rust no-return legacy functions and
floating-point expressions remain extensions; this packet does not claim complete
Java expression-language equivalence.

## Reference environment

All local Java and javac launches are disabled. New references use only Monkey
Man's `reference` wrapper in worker directory `worker.Bab5N6zr`. It pins the
campaign JAR, forces headless AWT and locale, and supplies the declared timezone.
`remote-environment.json` preserves root's setup manifest. Remote OpenJDK 21.0.5
and Graphviz 12.2.0 differ from origin Temurin 21.0.13 and Graphviz 16.1.0.
Root's seven conformance pairs establish only their measured scope.
Each new case has raw SVG/source/stdout/stderr, status and environment provenance
in `references/`. Exact source is supplemental; SVG comparisons use the shipped
CLI and unchanged strict comparator. Invalid references are negative controls,
never positive SVG evidence.

`ZoneNames.java` asserts headless mode and enumerates every JDK zone ID and both
standard/daylight SHORT/LONG names in en_AU. Its output is embedded locale data,
not diagram output. The exact invocation, data hash and runtime limitation are in
`../tim-zone-display-width/data-provenance.json`. Offset rules still use existing
libc support; an unknown host zone ID falls back to its abbreviation. No runtime
JVM dependency is introduced.

## Reproduction

The frozen checkout cohort is 949, plus 89 additions: 1038. It is not the parent
955/1044 denominator (six registry controls absent), nor parent 1449. New maker
controls are separate. `generate.py` preserves how their exact input bytes were
created; use the checked-in `inputs/` and `references/` for replay, without
regeneration or transport testing. Includes retain their relative hierarchy.

Build the CLI with `CARGO_TARGET_DIR=/tmp/rustuml-tim-repair-target` and
`CARGO_BUILD_JOBS=4`. Compile `probe.rs` against this checkout's release
`rustuml_oracle` rlib (the comparator code is unchanged). `measure.py` accepts
`TIM_COMPARATOR` and `TIM_MAKER_SCRATCH`. Under a fresh scratch directory, copy
`inputs/` to `new/`, `references/` to `reference/`, and `new-manifest.json` to its
root. Copy the candidate binary there as `replay-rustuml`. From the checkout root,
run `python3 <this-directory>/measure.py replay new` with both environment
variables set. It runs no Java; the `reference` phase is separate and unnecessary
when preserved references exist. Install PyYAML for supplemental YAML extraction.

For `isolated`, use the unchanged checker2 `parent-inventory.json`; only paths
present in this checkout are measured and the denominator is reported. For
`fresh`, extract the existing checker2 `measurement-artifacts.tar.gz` into
scratch `checker-archive/` to reuse exact Java source diagnostics. Set
`PLANTUML_STDLIB` to the pinned stdlib checkout; the measurement script records
its original absolute location. Named-zone rows supply matching `TZ` and remove
the fixed gate timezone override via their declared `debug` setting.

Final results, command status, regressions and pending dependencies are recorded
in the subsequent maker-results commit. No frozen input, reference, baseline,
review, comparator, guard allowance or error policy is changed.
