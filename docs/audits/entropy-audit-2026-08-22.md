# Entropy audit — rustuml (2026-08-22)

## Executive summary

- **Snapshot:** `/Users/marcelo/work/github.com/marcelocantos/rustuml`
  - Branch: `t14-ratchet` (tracking `origin/t14-ratchet`, **ahead 674**)
  - HEAD: `758bfec7e4955427a310115f3c3301f3c4c7fb89` (`758bfec7 Update bullseye.yaml`)
  - Date: 2026-08-22
  - Initial dirty state: **988** `git status --porcelain=v1` entries; **867** already staged (user-owned). Mix of `M`, `MM`, `D`, and ` D`. No untracked files at start. This audit cites **HEAD** unless a path is noted as working-tree-only. It does not treat the dirty tree as product truth.
- **Scope:** workspace crates, CI, Makefile, docs/governance, lockfile, scripts. Named exclusions below.
- **Headline mechanism:** the shipping renderer is one crate that owns both the product generative model and an always-compiled golden-replay/oracle-layout model, stuffed into 180k+ lines of per-diagram files. CI already ratchets no-oracle SVG parity, but several standing oracles that exist in-tree are not on the CI path.
- **Highest-consequence findings:** ENT-001 (dual generative models in `rustuml-render`), ENT-002 (megacrate / megafiles), ENT-003 (CI omits `doc_claims`, held-out perturbations, and the CLI binary journey).
- **Unverified residue:** `cargo test`, `cargo clippy`, and golden suites were not executed (dirty tree; running them would mix user-owned work into audit evidence). `cargo-audit` / `cargo-deny` / `cargo-machete` are not configured. Windows/cross-compile of Graphviz FFI not exercised.

## Scope and exclusions

**In scope**

- `Cargo.toml`, `Cargo.lock`, crate manifests and `src/`
- `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `Makefile`
- `AGENTS.md`, `CLAUDE.md`, `README.md`, `STABILITY.md`, `bullseye.yaml`, `docs/TODO.md`, `NOTICES`, `LICENSE`
- `scripts/` (4 POSIX sh files + `viewer.py`)
- Test *architecture* (harness, CI wiring, baseline files), not fixture contents

**Named exclusions (not silent omissions)**

| Tree | Role | Why skipped as product code |
|---|---|---|
| `test-diagrams/golden/` | git submodule `rustuml-golden` | ~12,500+ `.puml`/`.svg` oracle fixtures |
| `test-diagrams/perturbations/` | held-out T14 inputs | fixture corpus; wiring is in scope |
| `vendor/graphviz/` | vendored Graphviz C (246 tracked files, ~2.7M) | third-party layout engine; FFI wrapper is in scope |
| `crates/rustuml-parser/themes/` | bundled PlantUML themes | verbatim third-party data |
| `crates/rustuml-render/fonts/` | embedded TTF | binary assets |
| `target/` | build output | generated |
| `docs/parity-reviews/` | T14 maker/checker records | process artifacts; existence is a healthy control |
| Working-tree dirty files | user-owned in-flight work | 988 paths; not the audited snapshot |

Languages analyzed: Rust (product), C (vendored Graphviz + `build.rs` / FFI), POSIX sh (scripts), Python (`scripts/viewer.py` only). No Go/SQL/web frontend. Java files under `crates/rustuml-render/tools/` and `tools/` are metrics extractors, not the shipped path.

## Commands run

All from repo root. None of these is the shipped CLI path except where noted.

| Command | Version | Exit | Relevant output / limitation |
|---|---|---|---|
| `git rev-parse --abbrev-ref HEAD`; `git rev-parse HEAD`; `git status --porcelain=v1 -b` | git 2.55.0 | 0 | Branch `t14-ratchet`, HEAD `758bfec7…`, `## t14-ratchet...origin/t14-ratchet [ahead 674]`, 988 dirty lines, 867 staged |
| `git log -1 --pretty=format:'%h %s'` | git 2.55.0 | 0 | `758bfec7 Update bullseye.yaml` |
| `test -f hygiene.yaml` | — | 0 | **ABSENT** |
| `git ls-files '*.rs' \| xargs wc -l` | — | 0 | Working-tree `.rs` total 198,039 lines (includes dirty edits). Largest: `activity.rs` 40,342 |
| `git show HEAD:crates/rustuml-render/src/activity.rs \| wc -l` (and class/state/component/sequence) | — | 0 | HEAD: activity 40,342; class 20,258; sequence 13,998; state 12,985; component 10,126 |
| `git ls-tree -r HEAD --name-only crates/rustuml-render/src` line sum | — | 0 | 183,092 lines under render `src/` at HEAD (includes non-`.rs` assets) |
| `cargo metadata --format-version 1 --no-deps` | cargo 1.96.0 (30a34c682 2026-05-25) | 0 | Workspace DAG: `rustuml` → parser+render; render → layout+math+parser; oracle → render(`oracle-layout`)+parser. No crate cycle |
| `cargo tree -d --workspace --locked` | cargo 1.96.0 | 0 | Duplicate `resvg` 0.45.1 (via `svg2pdf`) and 0.47.0 (direct); duplicate `bitflags`, `gif`, `kurbo`, `imagesize`, `getrandom` |
| `rustc --version`; `cargo clippy --version`; `rustfmt --version` | rustc 1.96.0 (Homebrew) | 0 | clippy 0.1.96; rustfmt 1.9.0. No `cargo-nextest` on PATH |
| `git log --format= --name-only -n 400 -- '*.rs' \| sort \| uniq -c` | git 2.55.0 | 0 | Highest churn: `class.rs` 104, `state.rs` 80, `component.rs` 50, `parse/class.rs` 42, `layout/graph.rs` 35 |
| `git grep` (HEAD) for `oracle-layout`, `OracleLayout`, `canonical_skinparam`, `layout_full(`, silent golden skip | git 2.55.0 | 0 | Feature cfg only in `lib.rs` (4 hits). `render_with_oracle` present in every Svek-family renderer |
| `/Users/marcelo/.claude/skills/hygiene/hygiene_check.py` | uv-run script (pyyaml inline) | 1 | `FileNotFoundError: hygiene.yaml`. `--help` also crashes (no argparse; always `check_repo`) |
| `cargo test` / `cargo clippy --workspace` / golden suites | — | **not run** | Dirty tree (988 paths). Would compile user-owned work. Residue |

Shipped-path vs auxiliary: golden_no_oracle and `pipe_mode` are shipped-path oracles. `golden_pairs` / `render_svg_with_oracle` are auxiliary. This audit did not execute either.

## Observed architecture

```
rustuml (bin)
  ├─ rustuml-parser     TIM preprocess + typed Diagram AST
  └─ rustuml-render     SVG/PNG/PDF/EPS/ASCII
        ├─ rustuml-layout   vendored Graphviz FFI (cc build.rs)
        ├─ rustuml-math     katex/QuickJS
        └─ rustuml-parser
rustuml-oracle (test crate only)
  └─ rustuml-render with feature oracle-layout
```

**Declared rules that match observation**

- Semantic rewrite, not Java transliteration (`AGENTS.md`).
- Single binary, no JVM / Graphviz / fonts at runtime (`bullseye.yaml` T1; layout is statically linked C).
- Product metric is no-oracle SVG parity (`STABILITY.md`, `README.md`, `crates/rustuml/src/main.rs` `--help-agent`, `test-diagrams/no_oracle_baseline.txt`).
- Workspace split with `resolver = "2"` (`Cargo.toml` lines 1–8). Directional crate deps, no cycle.
- Parser owns `diagram/` (AST) and `parse/` (text → AST). Render consumes `Diagram`.
- Graphviz C is not thread-safe; `GRAPHVIZ_LOCK` serializes layout (`crates/rustuml-layout/src/graph.rs:17-19` at HEAD).
- T14 honesty guards exist (`crates/rustuml-oracle/tests/harness_guards.rs`) and run in CI.

**Observed rules inferred from code**

- Every Svek-family renderer exposes `render` + `render_with_oracle`. Product `render()` calls `render_with_oracle(..., None)` (e.g. `class.rs:2151-2165` at HEAD).
- `layout_oracle` is a **public** module of the shipping render crate (`lib.rs:48`) and is not `cfg`-gated. The `oracle-layout` feature only gates the crate-level dispatcher (`lib.rs:75-76, 149-150, 358-386`).
- CI on `master` push/PR only: fmt, clippy, `cargo build`, `cargo test --lib`, `harness_guards`, release golden tiers, live PlantUML oracle tests.
- CLI always writes stdout (`main.rs:133-137`); it never writes `input.svg` beside the input.

**Contradictions**

- `README.md:69` says `rustuml input.puml` “writes input.svg alongside input.puml”. The binary prints SVG to stdout. `--help` does not claim a sidecar file.
- `lib.rs:146-148` says oracle layout is “Currently supported for class diagrams; other types ignore the oracle.” `render_with_theme_and_oracle` dispatches ~18 variants to `render_with_oracle`.
- `AGENTS.md` / `CLAUDE.md`: preprocessor is “~2900 lines”. HEAD `preprocess/mod.rs` is 4,438 lines.
- `docs/TODO.md` is a live task list. Global `AGENTS.md` bans TODO files in favor of bullseye.
- Several achieved bullseye targets still have `acceptance: [TODO]` (`bullseye.yaml:40-41` for T1.1).

**Unknown intent (owner judgment)**

- Whether oracle-replay code should remain in the shipping crate for compile-sharing, or move behind `cfg` / into `rustuml-oracle`.
- Whether Svek-family renderers should share a module now, or stay split until more families de-oracle.
- Whether the CLI should grow PlantUML-like sidecar-file behavior or README should be corrected to stdout.

## Dimension vector

| Dimension | State | Evidence summary | Change from baseline |
|---|---|---|---|
| Architecture topology | concern | Clean crate DAG, but shipping render crate hosts a second generative model and parser internals leak into render (`state.rs` → `parse::canonical_skinparam_key`) | n/a (first full audit) |
| Redundancy / sources of truth | concern | Duplicate `cleanForKeySlow`; README vs CLI; TODO.md vs bullseye; dual `resvg`; dual oracle comments | n/a |
| Change amplification | critical | `rustuml-render` ~183k lines; `activity.rs` 40k; class/state/component co-churn; oracle+product in one function | n/a |
| Local code quality | concern | Linear PlantUML ports in megafiles; FFI isolated; not a helper-explosion problem | n/a |
| Correctness / verification | concern | Strong no-oracle ratchet in CI; `doc_claims`, perturbations, `pipe_mode` exist and are not CI-gated; golden tests no-op if submodule missing | n/a |
| Security / dependencies | concern | Vendored Graphviz + lockfile; Homebrew `skip_checksum: true`; no cargo-audit/dependabot; duplicate heavy SVG stacks | n/a |
| Build / release / operations | concern | rust-cache on CI test jobs; release matrix has no Windows; no `rust-toolchain.toml`; clippy not `--all-targets` | n/a |
| Documentation / governance | concern | T14 docs ratchet exists but is not in CI; `hygiene.yaml` absent; `docs/TODO.md` still live | n/a |

Do not aggregate these states into a scalar.

## Findings

### ENT-001: Shipping renderer hosts two generative models

- **Priority:** P1
- **Dimensions:** Architecture topology; Change amplification; Correctness / verification
- **Status:** observed fact
- **Evidence:**
  - Feature is documented as development-only (`crates/rustuml-render/Cargo.toml:7-9`) and is enabled only by `rustuml-oracle` (`crates/rustuml-oracle/Cargo.toml:13`).
  - Product binary does **not** enable the feature (`crates/rustuml/Cargo.toml:8-9`).
  - `pub mod layout_oracle` is unconditional (`crates/rustuml-render/src/lib.rs:48`). `#[cfg(feature = "oracle-layout")]` appears only four times, all in `lib.rs` (lines 75, 149, 358).
  - Product class render delegates into the oracle function: `pub fn render(...) { render_with_oracle(diagram, theme, None) }` (`class.rs:2151-2165` at HEAD). The `Some` branch replays golden SVG bodies verbatim (`class.rs:2176-2184`). The same `if let Some(oracle)` pattern exists in `component.rs:1134`, `deployment.rs:199`, `activity.rs:26518`, `mindmap.rs:532`, `gantt.rs:289`, `math.rs:50`.
  - Strict tests inject layout extracted from goldens (`crates/rustuml-oracle/tests/golden_pairs.rs:95-119`). No-oracle tests call `rustuml_render::render_svg` (`golden_no_oracle.rs:64-73`).
- **Mechanism:** a geometry change is implemented in a function that can take coordinates from Graphviz **or** splice PlantUML SVG. The strict tier can pass by exercising the `Some` branch while the CLI (`None`) still fails. That is the T14 gaming surface, now encoded as architecture rather than a one-off incident.
- **Blast radius:** every Svek-family and several non-Svek renderers; every parity fix; binary size and compile time of the shipped crate.
- **Counterevidence checked:** no-oracle ratchet is in CI (`ci.yml:50-51`). `harness_guards.rs:49-58` forbids embedding `test-diagrams` in shipping crates. Verbatim replay is gated on `oracle.is_some()`, so the CLI does not emit golden XML. The dual model is deliberate for the strict regression net.
- **Smallest coherent remediation:** `cfg`-gate `layout_oracle` and every `render_with_oracle` body so the shipping crate compiles only the `None` path; keep injection in `rustuml-oracle` via the feature. Stop calling product `render()` through the oracle function.
- **Verification:** `cargo test -p rustuml-oracle --test golden_no_oracle --release` plus a compile-time grep/guard that `crates/rustuml-render/src` without `--features oracle-layout` contains no `wrap_oracle_envelope` / `root_g_inner_xml` replay.
- **Ratchet candidate:** extend `harness_guards` with “oracle-replay APIs are cfg-gated”; CI job already present.

### ENT-002: `rustuml-render` is the compile-and-change megacrate

- **Priority:** P1
- **Dimensions:** Change amplification; Local code quality; Build / release / operations
- **Status:** observed fact
- **Evidence:**
  - HEAD line counts: `activity.rs` 40,342; `class.rs` 20,258; `sequence.rs` 13,998; `state.rs` 12,985; `component.rs` 10,126. Render `src/` tree 183,092 lines. Parser `src/` 33,485. Layout 3.6k. Binary `main.rs` 281.
  - Last 400 `.rs` commits: `class.rs` 104 touches, `state.rs` 80, `component.rs` 50 — the same files keep moving together.
  - `activity.rs` has two `#[cfg(test)]` regions (lines 30660 and 38585 at HEAD), so tests and the FTile port share one translation unit.
  - `rust.md` compile-unit rule: a one-file edit must not recompile the world. Editing any activity helper rebuilds a 40k-line module plus dependents.
- **Mechanism:** PlantUML diagram types were ported as whole files. Incremental `cargo check` and review cost scale with file size, not with the size of the behavioral change. Co-churn shows class/state/component fixes already land as shotgun edits.
- **Blast radius:** every renderer change, CI golden `--release` compile, agent compile loops (explicitly flagged in `AGENTS.md` model policy).
- **Counterevidence checked:** splitting by diagram type *across crates* would be the wrong seam (shared SVG/metrics/creole). In-file length is partly the Java model (FTile, Svek). Unit tests next to the port are a valid local oracle. This is not “extract helpers for cleanliness.”
- **Smallest coherent remediation:** split along already-named subsystems inside the crate (`ftile` is only 1,131 lines and only `activity.rs` uses it). Move `#[cfg(test)]` tails to `crates/rustuml-render/tests/` or sibling modules. Do not rewrite algorithms while splitting.
- **Verification:** `cargo build --timings` before/after a one-line `activity.rs` edit; check incremental rustc time. Architecture test: no `src/*.rs` file above an agreed line ratchet (start at 40,342 and only lower it).
- **Ratchet candidate:** `git ls-files '*.rs' | xargs wc -l` ceiling per file in `harness_guards` or a `make` target, ratcheted downward.

### ENT-003: Standing oracles exist and are not on CI

- **Priority:** P1
- **Dimensions:** Correctness / verification; Documentation / governance
- **Status:** observed fact
- **Evidence:**
  - CI `test-unit` runs `cargo test --lib` and `harness_guards` only (`ci.yml:34-37`).
  - CI `test-golden` runs `golden_pairs`, `golden_no_oracle`, `harness_guards` (`ci.yml:48-53`).
  - CI `test-oracle` runs `oracle`, `proptest_oracle`, `matrix_oracle` (`ci.yml:91-105`).
  - **Not referenced in `.github/workflows/`:** `doc_claims.rs` (T14.5 documentation ratchet, asserts `11,175/11,251` in README/STABILITY/`main.rs` at lines 274–311), `parity_perturbations.rs` (T14 held-out no-oracle perturbations, line 36), `crates/rustuml/tests/pipe_mode.rs` (only CLI binary journey), `crates/rustuml-parser/tests/error_paths.rs`, `preproc_*.rs`, `golden_formats.rs`, `golden_matrix.rs`.
  - Makefile `test` is also `--lib` only (`Makefile:16-17`).
  - `AGENTS.md` T14.10: “The no-oracle perturbation test must pass before acceptance.” There is no CI job that runs it.
- **Mechanism:** a README headline, a held-out perturbation, or a CLI-arg regression can land on `master` while CI stays green. The repo already paid for these oracles; they are unwired.
- **Blast radius:** published parity claims, T14 maker/checker gate, owner-visible CLI (`-pipe`, formats).
- **Counterevidence checked:** `golden_no_oracle` *is* the product SVG oracle and *is* in CI. `harness_guards` is in two jobs. `pipe_mode` is a thin wrapper over `render_svg`; SVG parity still covers rendering. `doc_claims` is cheap and should have been the easiest add.
- **Smallest coherent remediation:** add three steps to `test-unit` (or a small `test-gates` job): `doc_claims`, `parity_perturbations`, `pipe_mode`. Do not wait for a hygiene.yaml.
- **Verification:** a PR that changes the README total without the baseline, or that breaks `-pipe`, fails CI.
- **Ratchet candidate:** CI steps named above; later `hygiene.yaml` `ci_job` evidence.

### ENT-004: Svek-family layout is implemented N times

- **Priority:** P2
- **Dimensions:** Redundancy / sources of truth; Change amplification
- **Status:** inference (same engine, separate files; not a byte-identical clone)
- **Evidence:** `layout_full(Duration::from_secs(5))` callers at HEAD: `class.rs:2606`, `component.rs:1901`, `object.rs:597`, `state.rs:3372,5350,6360`, `usecase.rs:2026`, plus `deployment.rs:3821` (`LAYOUT_TIMEOUT`), `archimate.rs:663`, `dot_diagram.rs:130`. Activity uses 2s (`activity.rs:26762`). Each of class/component/deployment/object/usecase/state has its own `render` / `render_with_oracle` pair (grep `pub fn render` at HEAD). Churn ranking is the same set.
- **Mechanism:** a Graphviz/Svek invariant (hidden entities still occupy LimitFinder space, `constraint=false` on entry/exit, cluster ownership) must be re-applied per diagram type. History already shows that class fixes spawn state/component follow-ups (`docs/parity-reviews/` cluster).
- **Blast radius:** six renderers + layout FFI; T14 class/state/component work.
- **Counterevidence checked:** PlantUML itself has separate Svek backends per diagram; identical sharing would couple unrelated syntax. Types are not clones (usecase ovals vs class compartments). Shared pieces already exist: `rustuml-layout`, `plantuml_metrics`, `layout_oracle`.
- **Smallest coherent remediation:** extract only proven-shared Svek *mechanisms* (hidden-in-layout, cluster envelope, timeout constant) after a named parity-review, not a grand unified renderer.
- **Verification:** a perturbation that hides a node in class *and* component and asserts LimitFinder canvas behavior, run without oracle.
- **Ratchet candidate:** one shared timeout constant; architecture test that `layout_full` timeouts are not file-local literals.

### ENT-005: Two `SkinParam.cleanForKeySlow` ports

- **Priority:** P2
- **Dimensions:** Redundancy / sources of truth
- **Status:** observed fact
- **Evidence:** `crates/rustuml-parser/src/parse/mod.rs:36-61` (`pub fn canonical_skinparam_key`) and `parse/style.rs:1000-1020` (`fn canonical_skinparam_keys`) both lower-case, strip `_`/`.`, rewrite `sequenceparticipant`/`sequenceactor`, strip `<type>arrow`, map `align` → `alignment`. Render `state.rs` consumes the `parse` export (HEAD lines 2417–3544).
- **Mechanism:** a Java key-identity fix applied in one function can miss the other. Stereotype splitting exists only in `style.rs`, so the copies are already diverging by design without a shared core.
- **Blast radius:** every skinparam / theme diagram; state renderer in particular.
- **Counterevidence checked:** `style.rs` extra stereotype loop is a real additional concern, not accidental duplication of that part. Comments both cite `SkinParam.cleanForKeySlow`. Unit test `canonical_skinparam_key_matches_java_cleanup_rules` covers `mod.rs` only.
- **Smallest coherent remediation:** one function for the shared cleanup; `canonical_skinparam_keys` calls it then splits stereotypes.
- **Verification:** existing unit test plus a stereotype key that only `style.rs` handled, asserted equal after refactor.
- **Ratchet candidate:** `git grep -c 'sequenceparticipant' -- crates/rustuml-parser` equals 1.

### ENT-006: Documented CLI writes a sidecar file; binary writes stdout

- **Priority:** P2
- **Dimensions:** Documentation / governance; Correctness / verification
- **Status:** observed fact
- **Evidence:** `README.md:69-70` (`writes input.svg alongside input.puml`). `crates/rustuml/src/main.rs:133-137` `print!` of SVG. PNG/PDF/EPS also go to stdout (`139-170`). `pipe_mode.rs` asserts stdout. `doc_claims.rs` does not assert the sidecar sentence. STABILITY lists `rustuml <file>` as Stable without specifying destination.
- **Mechanism:** an owner following README looks for `input.svg` and concludes the tool failed. `--help` is correct; README is the page GitHub shows first.
- **Blast radius:** new users, agent integrations, any script that expects PlantUML’s write-beside-input default.
- **Counterevidence checked:** PlantUML itself writes sidecar files; rustuml chose stdout (pipe-friendly). Usage text and `--help-agent` describe stdout. Pre-1.0. Not a crash.
- **Smallest coherent remediation:** change the README line (and any other sidecar claim) to stdout, *or* implement sidecar writes when the input is a path and stdout is a TTY. Do not do both.
- **Verification:** `doc_claims` (or `pipe_mode`) fails if README claims sidecar writes while `main.rs` has no `File::create`.
- **Ratchet candidate:** extend `doc_claims.rs`.

### ENT-007: Two `resvg` stacks in one binary

- **Priority:** P2
- **Dimensions:** Security / dependencies; Build / release / operations
- **Status:** observed fact
- **Evidence:** `crates/rustuml-render/Cargo.toml:22-23` (`resvg = "0.47"`, `svg2pdf = "0.13"`). `cargo tree -d --locked` shows `resvg 0.45.1` via `svg2pdf` and `resvg 0.47.0` direct, plus paired `gif`/`kurbo`/`usvg`/`imagesize`. PNG is a third raster path (`png = "0.18"`).
- **Mechanism:** PNG and PDF rasterize through different resvg majors. A color/text bug fix in 0.47 does not apply to PDF. Compile time and binary size pay twice.
- **Blast radius:** `-tpng` vs `-tpdf`; release artifacts; `cargo build --release`.
- **Counterevidence checked:** svg2pdf 0.13 has not moved to resvg 0.47; this may be upstream lag, not local sloppiness. PNG via resvg 0.47 is the documented raster path (`NOTICES`).
- **Smallest coherent remediation:** when svg2pdf supports resvg 0.47, collapse; until then document the split as accepted and watch `cargo tree -d`.
- **Verification:** `cargo tree -d -p rustuml-render --locked` has one `resvg`.
- **Ratchet candidate:** CI step `cargo tree -d` allow-list.

### ENT-008: Homebrew release uploads with `skip_checksum: true`

- **Priority:** P2
- **Dimensions:** Security / dependencies; Build / release / operations
- **Status:** observed fact
- **Evidence:** `.github/workflows/release.yml:56-72` (`Justintime50/homebrew-releaser@v3`, `skip_checksum: true`). Formula test is only `--version`. No Windows matrix entry (`13-23`: darwin-arm64, linux-amd64, linux-arm64). Release job has no `Swatinem/rust-cache`.
- **Mechanism:** tap consumers cannot verify artifact integrity via checksum. A swapped binary would still install. `docs/TODO.md:17` still lists Windows binaries as undone; STABILITY claims a cross-platform single binary.
- **Blast radius:** every Homebrew install; supply chain of `marcelocantos/homebrew-tap`.
- **Counterevidence checked:** GitHub release assets are still produced (`gh release upload`). `contents: write` is expected for uploads. Checksum skip may have been a releaser workaround; it is still a standing hole.
- **Smallest coherent remediation:** stop skipping checksums; add `sha256` into the formula. Windows is a separate product decision.
- **Verification:** formula file contains sha256; releaser job does not set `skip_checksum`.
- **Ratchet candidate:** hygiene `file`/`ci_step` once `hygiene.yaml` exists.

### ENT-009: Golden suites pass if the corpus is missing

- **Priority:** P2
- **Dimensions:** Correctness / verification
- **Status:** observed fact
- **Evidence:** `golden_pairs.rs:10` documents silent skip. Implementation `golden_pairs.rs:182-185` and `golden_no_oracle.rs:113-116`: if `golden_dir()` lacks `sequence/`, eprint and `return` (test success). `harness_guards.rs:116` census also skips. CI `test-golden` does `submodules: true` (`ci.yml:43-45`), which usually populates the corpus.
- **Mechanism:** a local `cargo test --test golden_no_oracle` without `git submodule update --init` is green and measures nothing. A CI checkout that fails to recurse in a future workflow edit would ratchets-pass at 0 pairs.
- **Blast radius:** anyone running tests without the submodule; CI workflow edits.
- **Counterevidence checked:** current `test-golden` job requests submodules. Comment in `golden_pairs.rs` is honest. Empty-pair early return at lines 188-191 is the same hole.
- **Smallest coherent remediation:** fail (not skip) when `GOLDEN_REQUIRED=1` or when running in CI (`CI=true`). Keep skip for laptop clones without the corpus.
- **Verification:** `CI=true cargo test --test golden_no_oracle` with the submodule path renamed exits non-zero.
- **Ratchet candidate:** CI env `GOLDEN_REQUIRED=1`.

### ENT-010: Competing work ledgers

- **Priority:** P2
- **Dimensions:** Documentation / governance; Redundancy / sources of truth
- **Status:** observed fact
- **Evidence:** `docs/TODO.md` still has open High/Medium/Low items (lines 3–19) overlapping STABILITY 1.0 gaps and bullseye. Global agent rules ban TODO files and require bullseye. `bullseye.yaml` T1.1–T1.4 are `status: achieved` with `acceptance: [TODO]` (e.g. lines 36–41). `AGENTS.md:81` preprocessor size is stale (2,900 vs 4,438).
- **Mechanism:** an agent or human can “finish” a TODO line, a STABILITY checkbox, or a bullseye target independently. Achieved-with-TODO acceptance is an untestable target.
- **Blast radius:** planning, `/cv`, onboarding.
- **Counterevidence checked:** bullseye is clearly the intended SoT (`bullseye.yaml` banner). `doc_claims` already locks README/STABILITY *numbers*. `docs/TODO.md` “Done” sections are historical and useful.
- **Smallest coherent remediation:** promote any live `docs/TODO.md` bullets into bullseye (or STABILITY) and delete the file. Replace `acceptance: [TODO]` on achieved targets with the actual property or a pointer to a test.
- **Verification:** `test ! -f docs/TODO.md`; bullseye validate rejects `acceptance: TODO`.
- **Ratchet candidate:** `absent: file docs/TODO.md` in future hygiene.yaml.

### ENT-011: Hygiene posture is undeclared

- **Priority:** P3
- **Dimensions:** Documentation / governance
- **Status:** observed fact
- **Evidence:** no `hygiene.yaml`. Validator `/Users/marcelo/.claude/skills/hygiene/hygiene_check.py` exits 1 with `FileNotFoundError` on that path. CI still has real jobs; they are not declared as a posture vector.
- **Mechanism:** fleet query “which repos lack secret-scan / clippy / SBOM” cannot see rustuml. Drift of existing CI jobs is not mechanically compared to a floor.
- **Blast radius:** fleet hygiene only; in-repo CI still runs.
- **Counterevidence checked:** this audit must not initialize `hygiene.yaml`. Many controls already exist (LICENSE, CI cache, golden ratchet).
- **Smallest coherent remediation:** onboard hygiene in a later, requested change; floors matching reality (`correctness: 2` is plausible; `security` lower until checksums/audit).
- **Verification:** `hygiene_check.py` exit 0 against a committed yaml.
- **Ratchet candidate:** the yaml itself.

### ENT-012: `oracle-layout` crate docs are stale

- **Priority:** P3
- **Dimensions:** Documentation / governance
- **Status:** observed fact
- **Evidence:** `lib.rs:146-148` “Currently supported for class diagrams; other types ignore the oracle.” Dispatcher at `lib.rs:364-384` calls `render_with_oracle` for class, state, component, deployment, usecase, object, json, timing, gantt, salt, nwdiag, archimate, regex, ebnf, board, mindmap, wbs, activity, sequence, math.
- **Mechanism:** a later reader will under-estimate blast radius of oracle-layout changes.
- **Blast radius:** contributors touching `render_svg_with_oracle`.
- **Counterevidence checked:** comment may date from an earlier phase; code is the authority.
- **Smallest coherent remediation:** delete or replace the sentence with “feature-gated dispatcher; per-type functions decide None vs replay.”
- **Verification:** grep the stale sentence returns nothing.
- **Ratchet candidate:** none (comment).

## Redundancy and competing-source-of-truth inventory

| Fact | Authorities | Drift risk |
|---|---|---|
| No-oracle pass totals | `test-diagrams/no_oracle_baseline.txt` (SoT), README table, STABILITY, `--help-agent`, `doc_claims.rs` | Locked **if** `doc_claims` runs (ENT-003) |
| CLI output destination | `main.rs` (actual), README (wrong), `--help` (stdout) | ENT-006 |
| Skinparam key identity | `parse/mod.rs`, `parse/style.rs`, Java `SkinParam.cleanForKeySlow` | ENT-005 |
| Followable work | `bullseye.yaml`, `docs/TODO.md`, `STABILITY.md` gaps | ENT-010 |
| Layout geometry | Graphviz in-process vs golden `OracleLayout` | ENT-001 |
| Preprocessor size | `AGENTS.md` ~2900 vs `preprocess/mod.rs` 4438 | docs only |
| SVG rasterizer | resvg 0.47 (PNG) vs resvg 0.45 (PDF via svg2pdf) | ENT-007 |
| Theme application | `style.rs`, `style_cascade.rs`, `skinparam.rs` (2,979 lines combined) | not charged separately; three stages of one PlantUML pipeline |

Deliberate duplication: parser `diagram/` vs `parse/` (AST vs parser); golden SVG submodule vs product renderer; Java PlantUML as external oracle.

## Healthy structure worth retaining

- **Crate DAG without cycles.** `parser` has no render/layout/oracle deps. Oracle is a workspace member but not a binary dependency. This is the right product/test split at crate level (the leak is *inside* render — ENT-001).
- **T14 two-tier honesty, where it is wired.** `golden_no_oracle.rs:1-24` states ratchet semantics. Baseline file is checked in. CI `test-golden` runs both tiers plus `harness_guards`. Shipping crates allow-list for golden `include_str!` is empty (`harness_guards.rs:51`).
- **`doc_claims.rs` design** (even unwired): README table markers `<!-- no-oracle-status:start -->`, totals derived from the baseline, not hand-waved.
- **Graphviz isolation.** Vendored C, `cc` build, opaque FFI, process-wide mutex, timeout (`graph.rs:846-851` at HEAD) with grid fallback described in bullseye T1.2. Matches “no Graphviz binary at runtime.”
- **FFI timezone is cfg-gated.** `preprocess/debug.rs` HEAD: `#[cfg(unix)]` `localtime_r` vs `#[cfg(not(unix))]` UTC fallback — not a silent Windows compile break.
- **NOTICES + theme license filter.** `NOTICES` and `preprocess/themes.rs` exclude GPL-3 themes. Apache-2.0 for first-party code.
- **CI rust-cache** on check/unit/golden/oracle jobs (`Swatinem/rust-cache@v2`). Matches `rust.md`.
- **Parity-review + perturbation directories** as process, not as shipping code.
- **POSIX `#!/bin/sh` scripts** (`scripts/*.sh`) stay inside the bash.md “glue” bound; `generate-golden.sh` is 210 lines.

## Hygiene posture

`hygiene.yaml` is **absent**. Posture: **not declared**. This audit did not initialize one.

Validator run (mandatory explicit pass):

```
$ /Users/marcelo/.claude/skills/hygiene/hygiene_check.py
FileNotFoundError: [Errno 2] No such file or directory:
  /Users/marcelo/work/github.com/marcelocantos/rustuml/hygiene.yaml
exit 1
```

Reality that a future yaml would describe (not declared, not ratcheted):

| Dimension | Observed (undeclared) |
|---|---|
| correctness | `cargo test --lib` + golden two-tier on CI; missing ENT-003 tests |
| quality | `cargo fmt --check`, `cargo clippy -- -D warnings` (not `--all-targets`) |
| deps | `Cargo.lock` committed; no dependabot/cargo-audit/deny |
| release | GitHub Actions release + Homebrew; `skip_checksum: true` |
| security | no secret scan / CodeQL / SBOM in `.github/` |
| docs | README/LICENSE/STABILITY; `doc_claims` unwired |
| vcs | `.gitignore` present; no CODEOWNERS |
| agent | `AGENTS.md`/`CLAUDE.md`/bullseye present |

Overlap with entropy: ENT-003, ENT-008, ENT-009, ENT-011 are the hygiene-shaped items. ENT-001/002 are architectural and should not be reduced to a yaml checkbox until a compile/cfg guard exists.

## Oracle coverage and residue

| Property | Decides it today | Gap |
|---|---|---|
| No-oracle SVG parity per family | `golden_no_oracle` + `no_oracle_baseline.txt` (CI) | Silent skip if submodule missing (ENT-009) |
| Oracle-assisted SVG regression | `golden_pairs` (CI, auxiliary path) | Same skip; dual model (ENT-001) |
| Harness honesty (no golden embed) | `harness_guards` (CI) | Does not cfg-gate oracle replay |
| README/STABILITY totals | `doc_claims` (exists, **not CI**) | ENT-003 |
| Held-out perturbations | `parity_perturbations` (exists, **not CI**) | ENT-003 |
| CLI `-pipe` / binary | `pipe_mode` (exists, **not CI**) | ENT-003 |
| Live Java PlantUML parse/render | `oracle` / `proptest_oracle` / `matrix_oracle` (CI, needs JAR+server) | Not the product path |
| PNG/PDF/EPS | `golden_formats` (exists, **not CI**); smoke-level per TODO | Raster quality unratcheted |
| Preprocessor | `preproc_*` tests (**not CI**) | |
| Graphviz timeout/fallback | unit tests in `graph.rs` (`cargo test --lib`) | |
| Windows binary / tz names | `cfg(not(unix))` UTC fallback | No CI target |
| Secret scan / advisory audit | nothing | |
| Clippy on tests/examples | nothing (`--all-targets` unused) | |
| Incremental compile budget | nothing | ENT-002 |
| Full `cargo test` / clippy on this snapshot | **not run** (dirty tree) | residue |
| `cargo-nextest` | not installed | residue |

Owner-residue (intent, not mechanical leftover):

1. Should oracle-replay live in the shipping crate at all, or only behind `oracle-layout`?
2. Sidecar-file CLI vs stdout-only — pick one contract before 1.0.
3. Windows release artifacts: accept absence or add a matrix row.
4. How far to share Svek code across class/component/state without recreating a Java god-object.

## Remediation sequence

1. **Wire the oracles you already have (ENT-003, ENT-009).** CI: `doc_claims`, `parity_perturbations`, `-p rustuml --test pipe_mode`; `GOLDEN_REQUIRED=1` on `test-golden`. This is the enforcement seam; no renderer rewrite.
2. **Correct the CLI contract in README or code (ENT-006)** and extend `doc_claims`. Cheap, owner-visible.
3. **Cfg-gate oracle replay (ENT-001)** so the shipped crate compiles one generative model. Keep `golden_pairs` on the feature. Re-run no-oracle + guards.
4. **Single `cleanForKeySlow` core (ENT-005)** before the next skinparam campaign.
5. **Split megafiles without behavior changes (ENT-002)** starting with `activity.rs` test tails; ratchet line counts down. Only then extract proven Svek shares (ENT-004).
6. **Release hygiene (ENT-007, ENT-008):** checksums on; track dual-resvg until upstream aligns.
7. **Governance (ENT-010, ENT-011, ENT-012):** delete or promote `docs/TODO.md`; fill achieved-target acceptance; onboard `hygiene.yaml` only when asked; fix the stale oracle comment.
8. Re-run this audit against the same dimension definitions and HEAD identity.

No architectural rewrite is required to close the P1 CI gap.
