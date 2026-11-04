# Independent style/WBS corrective review

**Rejected for all five mechanisms.** This is a new independent checker record,
not a maker attestation. The local origin and Display projection have bounded
support, but fresh valid whole-output failures remain. None of those failures
is a newly lost pass.

Reviewed evidence tip `a3afe487a66ae26c73e2f1853d6c26fc2d228875`, production
`8df6bb53fac4fdf364a1a9aeea0b203cb6dab8e6`, against exact clean base
`a84856637adcd8e8d4e78fd2ee00658c9c612feb`. Reviewer: GPT-6 Astra, high,
Codex harness in T3 Code; new isolated task with no maker involvement or children.
The clean checkout was switched to `review/style-wbs-independent-20261105`.
No production, account, comparator, baseline, skip, allow-list, target or old
review was changed. There was no shipping or import of concurrent sequence work.

## Actual CLI replay

Independent empty private targets built exact `git archive` source snapshots
with jobs 4. Every source ran through the actual CLI `-pipe` and separately
`--yaml -pipe`, using the original cwd/include/environment contract. The unchanged
`compare_svg_strict` diagnostic was rebuilt from the reviewed source. All raw
models, SVGs, stderr, comparisons, exit statuses and hashes are archived.

| Cohort | Entries | Valid | Before | After |
| --- | ---: | ---: | ---: | ---: |
| retained | 1730 | 1609 | 709 | 709 |
| maker18 | 18 | 17 | 13 | 13 |
| sprites50 | 50 | 49 | 49 | 49 |
| checker38 | 38 | 34 | 26 | 26 |
| fresh15 | 15 | 15 | 13 | 13 |
| fresh26 | 26 | 25 | 19 | 24 |
| new15 | 15 | 14 | 3 | 12 |
| delimiters4 | 4 | 4 | 0 | 0 |
| focused2 | 2 | 2 | 0 | 2 |
| **Total** | **1898** | **1769** | **832** | **848** |

All denominators, duplicate observations and nonpositives are retained. There
are zero lost passes and no new errors or panics. The same 32 retained SVG
panics remain. Retained SVG exits are **1605/93/32** for exit 0/1/101; YAML
exits are **1622/93/15**, unchanged before/after. All other cohort CLI calls
exit 0. All 207 available valid retained and 17 maker evaluated-source
comparisons are exact.

Full YAML trees change only in the five reported WBS diagrams: twelve nodes
gain `shape: None`, with command syntax removed from labels, and one color
moves into its field. No other semantic model change occurred. This replay
observed no additional byte-only serialization difference; the maker archive's
single sprite-map order change is semantically equal, not a stable required
byte difference. A separate nested-model executable confirms absent shape
loads as Box, Box is omitted on serialization, and explicit None round-trips.

## Fresh independent challenges

Both immutable relay batches and all 38 source/reference pairs are retained.
The initial batch has 30 entries, 26 clean positives, and **2 → 20** passes.
The supplementary batch has eight entries, seven clean positives, and
**0 → 1** passes. Combined: **21/33 clean positives**, no lost passes, and all
76 before/after SVG calls and 76 YAML calls exit 0 without panics. One positive
failure (`display_forced_alignment`) is explicitly excluded from the verdict
as instructed, leaving **21/32 verdict-eligible positives**.

Five outputs are nonpositive, without rewriting their original intended-valid
labels:

- `shape_direction_new` and `shape_direction_old`: Java syntax-error SVGs,
  exit 200. They are not valid diagram comparisons and do not require Rust
  to panic. Both raw error outputs remain executable fixtures.
- `html_url_math`, `display_raw_math`, and `s_display_latex_valid`: the pinned
  runner lacks `org.apache.batik.dom.GenericDOMImplementation`; Java embeds
  fallback images. The first WBS math probe also contains an invalid latex
  command. Even the valid `\nabla`/`\nu` probe reaches the missing-Batik
  fallback. These are reference-environment limitations, not evidence of
  normal Java math behavior. All sources, streams and images remain retained.

The initial automated classification based on outer SVG error text is retained
as preliminary evidence; the final classification inspects stderr and embedded
images. The reference environment must not be inferred for historical goldens.

### Causal findings and verdicts

| Mechanism | Verdict | Bounded support and remaining counterexamples |
| --- | --- | --- |
| HTML style candidate eligibility | Rejected | Nested closed/EOL selection, aliases, Unicode and activity/state/note consumers pass. `\~` remains in captured text; empty size/color commands merge adjacent text runs; extended activation forms remain unsupported. |
| Delimited style candidate eligibility | Rejected | All five mid-line delimiter families and an activity consumer pass. Escaped overlapping openers consume two characters instead of Java's one; escaped tildes retain a backslash. |
| WBS Display line projection | Rejected | Blank-line placement, mixed sizes, depth/sides, escaped backslashes and raw URL projection pass. Fresh multiline empty-parameter and escaped-style cases still fail whole output through shared text. |
| WBS shape block selection | Rejected | New-order color/shape, whitespace-literal underscores and model round-trips pass. Old-order warning decoration is still missing. |
| WBS leaf component origin | Rejected | Six siblings, depth five and sizes 6/29/9 pass. The origin arithmetic is source-correct; short leaf/composed labels still fail shared-text flush/escape cases. |

Representative exact counterexamples are unpacked under `counterexamples/`.
Every case is also in the archive and registered beneath its mechanism's
`test-diagrams/perturbations/.../checker-independent-20261105/` directory.

`CharHidder.hide` consumes backslash-tilde before matching and recursion. Rust's
new candidate scanners recognize that escape for closing-boundary search, but
`walk_segments` leaves the backslash in the painted body. Java emits `a~`, Rust
`a\~`. For escaped overlapping delimiters, Java `q~***x**r` produces literal
`q*`, bold `x`, then `r`; Rust emits the whole `q***x**r` literally because its
existing tilde arm consumes two characters.

Java flushes pending text for every selected command. Rust's unchanged
size/color arms add no boundary for consumed-empty bodies. In
`s_display_commands`, both canvases are 140×191 and line positions agree, but
Java emits separate `q`/`r` atoms and Rust emits `qr`. `s_leaf_commands` shows
the same failure at matching positions in a 103×144 canvas. This supports the
projection/origin model while rejecting integrated whole-output acceptance.

All those fresh failures also failed before the packet. They are remaining
obligations, not fabricated regressions.

## Once-only broad gates

The candidate remained unchanged. Gates ran once at the evidence tip before
checker-only files were added; they were not repeated after those additions.
The golden submodule was hydrated locally at exactly
`9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`, with no setup failure.

- Parser 624, CLI pipe 15, layout 41, math 10 and oracle 31 passed.
- Renderer **782/784**; the same plain-theme class and initial-blank
  source-identity failures. Their actual CLI SVG and YAML inputs are
  byte-identical before/after.
- Main product library diagnostic **11179/11251**; assisted library diagnostic
  **10497/11251** (12550 total, 1299 skipped). Both are red. Neither is an
  actual-CLI truth metric.
- Standing **712/1766**, matching-environment actual CLI **742/1766**, before
  **726/1766**. The exact same 30 environment/include discrepancies remain.
- Guards **4/6**, with decimal ratchet and review-record guards red.
- Clippy fails at unchanged `parse/component.rs:72`; workspace fmt fails in
  unchanged class files. All four touched files format cleanly; diff check passes.

Registration is separate from repair: the older 1745 standing members contribute
698 harness passes and actual CLI **723 → 728**; the 21 newly registered members
contribute 14 harness passes and actual CLI **3 → 14**. The 38 checker additions
were not included in the once-only 1766-member broad run. Their actual CLI
comparisons are fully captured; no post-evidence standing-green claim is made.

## Provenance and preservation

All 29 files in the maker's Java source record match pinned source
`71806a23780b04a5ccde2f8ceb5121edad5eb711`; seven additional cited/supporting
files were independently checked. Jar SHA256 is
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Only the parent's existing Monkey Man runner executed Java. No local JVM,
javac, SSH connection, runner, child or additional worktree was created.

Accounts `829154e5b`, `52203dbe7` and `3da745665` precede their committed drafts
in Git ancestry, and archived edit/build timestamps are consistent with that
order. The initial style/WBS edits follow the first account, leaf final logs
follow its account, and delimiter edits follow theirs. Archive mtimes are
maker-controlled corroboration; they cannot independently prove the absence
of an unrecorded earlier draft. Exact times and commit bodies are in
`chronology.json`.

Production changes are exactly renderer `creole.rs`, `wbs.rs` and parser
`diagram/wbs.rs`, `parse/wbs.rs`. Public text signatures, `text_render.rs`,
legacy `to_svg_tspans`, class and sequence are unchanged. The leaf branch and
0/3/1/1 margins have named source provenance; no fixture-shaped logic or new
unexplained layout constant was found.

All **9731 old**, **118 earlier rejected-evidence**, and **96 newly imported**
files retain exact bytes. An additional 10172 base-tracked evidence artifacts
were compared by Git blob. The imported rejection's content is unchanged.
The maker archive's **64578 members** all match its inventory and recorded
SHA256 `680d863a30d2be60bc4302fc6b4b61aa061d73f78e9086a80985fc4cdde977b7`.

Both Java transport envelopes, original requests/manifests, lossless parent
mappings and raw streams were preserved. Hashes are in `archive-files.json`;
archive read-back verification is in `archive-integrity.json`. Java argv is
**derived from the wrapper, not process-traced**. Historical font/Graphviz
execution equivalence remains unproved.

## Remaining obligations

1. Implement one source-faithful escape/hiding stream for candidate selection
   and recursive painting, including backslash-tilde and overlapping escaped
   delimiter openers; preserve the new counterexamples.
2. Preserve Java command flush boundaries for empty parameter-driven commands,
   then revalidate the WBS consumers. Do not retune the supported leaf origin
   or line geometry to compensate for lexical failures.
3. Integrate old-order WBS warning metadata/decoration. Keep shape parsing,
   whitespace literals and serialization compatibility as separate invariants.
4. Retain the sequence mixed-size and line-stripe obligations. The original
   baselines reproduce exactly: Rust 93.6777 versus Java 106.2461 and 105.2793;
   all four original delimiter controls still fail.
5. Restore a separately pinned, complete reference environment for any future
   math assertion; preserve these limited references without treating them as
   normal Java math output or assuming historical goldens used this environment.

Unknown/guillemet, Unicode/font/PLAIN, note resolver, URL/italic/legacy tspan,
welcome and negative diagnostics remain executable in the immutable cohorts.
`malformed_dot` is a Java NumberFormatException SVG with exit 0 and remains
nonpositive. The class candidate
`68ab512be964e243ae1943f7a9fefd3537091401` is unintegrated: its quoted-label
split differs from this tested base. No combined class or concurrent sequence
correctness is asserted. `residuals.json` identifies the captured cases and
mixed-size observations.

## Reproduction

Extract `measurements.tar.gz` into a private cache. It includes frozen input
bytes, references and available evaluated-source outputs under `inputs/`, plus
all raw CLI outputs, JSON models, commands, environments and comparison streams.
`capture-paths.json` maps original contracts to archive-relative captures.
Rebuild the exact before/after revisions with the commands in
`build-provenance.json`; rebuild `maker/probe.rs` from the unchanged maker
archive using `diagnostic-build.json`. Replay scripts reuse that archive's
unchanged CLI driver and comparator. Restore/map recorded cwd resources
losslessly; historical resource archives are unchanged in the parent evidence.

`fresh_check.py` and `supplement_check.py` validate returned references and run
CLI comparisons; they never execute Java. `adjudicate.py` preserves preliminary
classification and records the embedded-image environment limitation. Do not
run the broad gates again merely to reproduce evidence-only packaging.
