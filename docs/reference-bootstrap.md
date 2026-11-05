# Pinned reference and verification gates

The recovery campaign uses Java PlantUML source revision
`71806a23780b04a5ccde2f8ceb5121edad5eb711` (`1.2026.3beta6`) and the golden
submodule at `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`. Preserve existing
references. Building a new JAR does not authorize regenerating any corpus.

## Owner-selected input contract (3 November 2026)

RustUML parity follows Java's raw CLI/file input behavior. The owner explicitly
approved this choice after clarification that the observed blank-line differences
affect internal SVG identifiers and source-line metadata, not diagram appearance.
Preserve outer and internal body blank lines before preprocessing; do not apply
picoweb GET's `DiagramSourceCompressor.decompress` normalization to raw input.

The historical golden corpus was generated through picoweb GET. Keep those
sources and references unchanged, and continue reporting their regression counts
with that provenance. They are not authoritative raw-reader references where the
front ends differ. New raw-reader evidence must identify the exact source bytes,
Java revision/JAR, input front end and environment. Compare source origins and
SVG identifier/reference consistency against that same raw front end, separately
from the historical GET result. A visually unchanged diagram does not license
broken SVG references or arbitrary source-location changes.

The affected parser origin assertion now expects 5/8/9 under the approved raw
contract, rather than GET's 1/4/5. That assertion update accompanies the TIM
candidate, which remains subject to independent checking. The three documented
historical mismatches are `activity/act_skinparam2_shadowing_true`,
`activity/act_skinparam_shadowing`, and
`edge-cases/edge_whitespace_multiple_blank_lines`; their raw counterparts must
be preserved as additional evidence, never replacements for the historical files.
The exact reader trace and reproduced outputs are in
`docs/parity-reviews/tim-evaluated-event-stream/checker-support/README.md`.

This decision does not widen SVG comparator tolerances, remove error markers,
change eligibility, or turn historical mismatches into passes. XML formatting
whitespace is not a visual parity requirement; meaningful text whitespace,
geometry, styling and valid identifier references remain required. Until a
separately reviewed raw-reference tier is established, publish both the unchanged
historical counts and explicitly scoped raw-reader results without conflating them.

## Provenance observed on 3 November 2026

- All 12,428 golden SVG files containing a PlantUML processing instruction name
  `1.2026.3beta6`; 126 other `.svg` files have no such instruction. These file
  counts include four SVGs without a paired source and are not eligibility counts.
- 1,115 golden files carry the full version text
  `1.2026.3beta6 / 71806a2 [2026-03-21 13:49:45 UTC]`.
- Preserved checker records, including
  `docs/parity-reviews/class-package-direct-standalone-layout/review.json`, name
  that source revision and historical JAR SHA256
  `c4d7d78bf9bc1145299d02f648d7895d4056dc262a091d25c91714dec0478d45`.
- The restored source's `gradle.properties` declares `1.2026.3beta6`.
  The CI reference therefore builds this source instead of downloading the
  unrelated historical CI pin `1.2025.2`.
- The campaign rebuild has SHA256
  `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
  It reports commit `71806a2`, compile time `2026-11-03 06:34:50 UTC`.
  This is **not** the historical binary. Upstream `build.gradle.kts` injects
  `System.currentTimeMillis()` into `CompilationInfo.java` and generates
  `git.properties`; rebuilding the same source does not reproduce its bytes.

## Restore without disturbing an existing reference

If the paths already exist, inspect `git status` and `git rev-parse HEAD` first.
Do not reset or overwrite someone else's checkout. The campaign restored these
previously absent reference directories; other workers must treat them as read-only.

```sh
git clone --filter=blob:none --no-checkout https://github.com/plantuml/plantuml.git \
  ~/work/github.com/plantuml/plantuml
git -C ~/work/github.com/plantuml/plantuml checkout --detach \
  71806a23780b04a5ccde2f8ceb5121edad5eb711

git clone --filter=blob:none --no-checkout https://github.com/plantuml/plantuml-stdlib.git \
  ~/work/github.com/plantuml/plantuml-stdlib
git -C ~/work/github.com/plantuml/plantuml-stdlib checkout --detach \
  747d1eae809cde20675976d67e3eb84dc0021a0c

export JAVA_HOME=/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home
export PATH="$JAVA_HOME/bin:$PATH"
cd ~/work/github.com/plantuml/plantuml
./gradlew :build -Pfast --no-daemon --max-workers=3
export PLANTUML_JAR="$PWD/build/libs/plantuml-1.2026.3beta6.jar"
shasum -a 256 "$PLANTUML_JAR"
java -Djava.awt.headless=true -jar "$PLANTUML_JAR" -version
```

Use the installed JDK path appropriate to the machine. On the campaign Mac,
`java` through the default jenv shim fails with `version '21' is not installed`,
despite the usable Temurin installation above. The Gradle wrapper selects 9.3.1;
`:build -Pfast` builds the root JAR without upstream tests or other license variants.
CI caches by source revision, runtime family and OS, and prints the actual JAR
hash and environment when it runs.

## Standard-library dependency

Set `PLANTUML_STDLIB` explicitly for Rust tests and product comparisons:

```sh
export PLANTUML_STDLIB="$HOME/work/github.com/plantuml/plantuml-stdlib/stdlib"
```

The pinned stdlib revision is the last commit preceding the pinned Java commit.
More specifically, all three `output/tupadr3/*.sha1` markers exactly match Java's
`src/main/resources/stdlib/tupadr3/*.sha1`:

| Bundle | Recorded SHA1 marker |
| --- | --- |
| info | `9e084a1a40cc1a03da15319bcc9e9b330f48f6aa` |
| puml | `a19d01af7b2341028354f509705093769f0d1f03` |
| sprite | `658f814fe7fd80ebbbff1be859cbe9e4adbdf3e1` |

These are upstream content markers, not newly computed SHA256 values. This
establishes tupadr3 provenance, not full equality of every bundled library.
Its README declares version `3.0.0`.

`deployment/deploy_sprite_server.puml` includes `tupadr3/common` and
`tupadr3/font-awesome/server`. The former supplies `hide stereotype`; the latter
defines the sprite and macros. Without the checkout, Rust silently drops those
includes and reserves a stereotype text row. Restoring the pinned files fixes
the observed deployment unit failure without changing the parser or renderer.
The rebuilt Java JAR's output for that source is identical to the preserved
golden after XML parsing (including geometry/text, excluding XML processing
instructions and comments). No golden was rewritten.

## Environment and limits of reproducibility

The observed reference runs on macOS arm64 with Temurin OpenJDK
`21.0.13+9-LTS`, UTF-8, language `en`, country `AU`, headless AWT, and external
Graphviz `16.1.0 (20260904.0139)`. Invoke Java with
`-Djava.awt.headless=true`; keep the Graphviz executable on `PATH`.

Java resolves fonts through the host. On this Mac at size 14, plain font,
antialiasing and fractional metrics enabled, `AppServer` has advance `69.125`
under `SansSerif`, `66.14453125` under `Arial`, `60.6484375` under
`Times New Roman`, and `75.6123046875` under `Courier New`. `Courier` falls back
to `Monospaced` here. These are environment diagnostics, not new renderer
constants. Fontconfig's `fc-match sans-serif` reports Verdana on this machine,
so that command alone does not establish Java AWT's logical-font mapping.

CI installs Graphviz and Liberation fonts and reports its environment. Linux
font substitution and the installed Graphviz version are not asserted to match
the historical macOS reference environment. Preserve failures and investigate
their provenance; do not regenerate expected SVGs to conceal environment drift.
The existing font/licensing decision remains outside this setup packet.

The golden harness pins date-dependent Rust output with
`RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100`. New Java references involving
dates need an explicit equivalent reference strategy; current wall-clock output
is not a replacement for the preserved date goldens.

`/tmp/rustuml-campaign-reference.json` records the ready local source/JAR paths,
hash, executable paths, environment, stdlib evidence and smoke result. This is
machine-local scratch evidence, not a portable checkout or a checker verdict.

## Standing gates

### Campaign reference execution on Monkey Man

At the owner's request, new campaign Java invocations run on Monkey Man, not
the origin Mac, where repeated JVM launches interrupted desktop focus. This
includes source probes and helper programs. Rust builds and comparisons against
existing references may continue locally.

The operational runner is `/Users/marcelo/rustuml-reference/bin/reference` on
`marcelo@192.168.1.112`, accessed through existing SSH trust and credentials.
The exact campaign JAR was transferred and its SHA256 verified. The wrapper
checks that hash on every launch, forces headless AWT, UTF-8 and `en_AU`, uses
absolute Java/dot paths, and preserves raw stdin, stdout, stderr and exit status.
It does not expose an HTTP service or apply GET source normalization.

Allocate an isolated remote directory per worker/job:

```sh
ssh -T -o BatchMode=yes marcelo@192.168.1.112 \
  /Users/marcelo/rustuml-reference/bin/new-worker
```

Use the returned path as `WORKER_PATH` below, replacing the placeholder before
execution. Stage inputs and includes with SCP/SFTP while preserving their relative
hierarchy. Never share a mutable directory between concurrent jobs.

```sh
ssh -T -o BatchMode=yes marcelo@192.168.1.112 \
  '/Users/marcelo/rustuml-reference/bin/reference --cwd WORKER_PATH --timezone GMT+11:00 -- -pipe -tsvg' \
  < input.puml > output.svg 2> output.stderr
```

Timezone is explicit when required by the reference; omit `--timezone` only when
the declared experiment uses the host default (`Australia/Melbourne`). File mode
uses `-- -tsvg input.puml` in the staged directory. External stdlib/include files
must be staged from the pinned source, not silently replaced by another version.
Collect outputs before deleting a job's directory. SSH failures are distinct
from PlantUML's status.

Java source probes are staged remotely and use `--probe REMOTE_SOURCE.java`.
For probes importing PlantUML, add `--classpath REMOTE_HELPER_DIRECTORY`, which
includes the pinned JAR. Probe arguments follow `--`. No origin-side JVM is
needed to compile or run the probe. The complete invocation contract and live
readiness record are in the remote operational directory's `README.md` and
`readiness.json`.

The existing remote runtime is Homebrew OpenJDK 21.0.5 and Graphviz 12.2.0 on
macOS arm64, differing from the origin versions above. No installs or global
changes were needed. Seven supplied source/reference pairs passed strict parsed
XML equality, including both sequence font controls, a nested sequence group,
class/container layout and the three raw-reader historical counterparts. This
calibration retains geometry, metadata and IDs; it ignores only XML declarations,
comments and processing instructions. It does not establish universal environment
equivalence. Record these runtime versions with new references and investigate
any new environment-dependent discrepancy without rewriting preserved evidence.

### Local Rust verification

From the RustUML checkout, after initializing its actual golden submodule:

```sh
git submodule update --init test-diagrams/golden
CARGO_BUILD_JOBS=4 make gate
```

Each worker must use its own Cargo target directory. `make gate` runs formatting,
Clippy, workspace library tests and `make test-parity`, continues independent
checks after failures and returns nonzero if any fail. `make test-parity` runs
both parity tiers, honesty guards, documentation claims, held-out perturbations
and CLI pipe tests with Cargo's `--no-fail-fast`. It rejects `GOLDEN_FILTER` and
`HARNESS_WRITE_BASELINES` so a diagnostic subset or baseline write cannot stand
in for the full gate. CI calls the same parity gate.

Absent, empty or unreadable golden and perturbation directories fail. Negative
tests cover missing/empty directories and Makefile exit masking. These setup
checks do not change eligibility, error markers, comparator tolerances, park
lists, baselines or checker verdicts. A red product or evidence suite remains red.
