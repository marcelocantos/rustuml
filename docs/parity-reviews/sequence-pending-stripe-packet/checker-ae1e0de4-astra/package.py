# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,json,hashlib,shutil,tarfile,subprocess,re,collections
from pathlib import Path
C=Path(__file__).parent;R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-78c595f4');D=R/'docs/parity-reviews/sequence-pending-stripe-packet/checker-ae1e0de4-astra';D.mkdir(exist_ok=False)
sha=lambda b:hashlib.sha256(b).hexdigest()
def read(p):return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
def write(p,x):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(x,indent=2)+'\n')
summary=read(C/'summary.json');fresh=read(C/'fresh86/measurements.json.gz')+read(C/'followup10/measurements.json.gz');old=read(C/'replay2169/measurements.json.gz');findings=read(C/'findings.json')
summary['fresh96']=dict(entries=len(fresh),distinct_source_bytes=len({x['source_sha256'] for x in fresh}),observed_valid=sum(x['observed_valid'] for x in fresh),before=dict(collections.Counter(x['results']['before']['cli_comparison']['exit'] for x in fresh)),after=dict(collections.Counter(x['results']['after']['cli_comparison']['exit'] for x in fresh)),recoveries=[x['source'] for x in fresh if x['results']['before']['cli_comparison']['exit']!=0 and x['results']['after']['cli_comparison']['exit']==0],losses=[x['source'] for x in fresh if x['results']['before']['cli_comparison']['exit']==0 and x['results']['after']['cli_comparison']['exit']!=0],new_valid_parse_rejections=[x['source'] for x in fresh if x['observed_valid'] and x['results']['before']['cli']['exit']!=1 and x['results']['after']['cli']['exit']==1],renderer_panics=[x['source'] for x in fresh if 'panicked at' in x['results']['after']['cli']['stderr']],ascii_changes=sum(x['results']['before']['ascii']['sha256']!=x['results']['after']['ascii']['sha256'] for x in fresh))
summary['welcome_handoff']=dict(entries=2,reference_commit='0f00130a26b2f300996d0bd427f9e70b4ef10e30',classification='Historical welcome-truncation outputs are nonpositive, not two more valid SVG failures or successes. Both Rust revisions render Class exit0; unchanged comparator exits2. Exact historical raw evaluated sources and full SVG/AST/ASCII captured separately.',additive_to_frozen=False,additive_to_fresh=False)
summary['fresh_mechanisms']={m:dict(entries=len(rr),valid=sum(x['observed_valid'] for x in rr),before=dict(collections.Counter(x['results']['before']['cli_comparison']['exit'] for x in rr)),after=dict(collections.Counter(x['results']['after']['cli_comparison']['exit'] for x in rr)),verdict=findings[m]['verdict']) for m in findings for rr in [[x for x in fresh if x['mechanism']==m]]}
write(C/'final-summary.json',summary)
# Preserve extra source provenance used to disprove ownership/rendering composition.
java=Path('/Users/marcelo/work/github.com/plantuml/plantuml');jr='71806a23780b04a5ccde2f8ceb5121edad5eb711';extra=[]
for path in ['sequencediagram/Participant.java','sequencediagram/graphic/DrawableSetInitializer.java','sequencediagram/graphic/Step1MessageExo.java','sequencediagram/graphic/Step1Message.java','sequencediagram/teoz/CommunicationExoTile.java','sequencediagram/teoz/CommunicationTile.java','skin/rose/ComponentRoseQueue.java','skin/AbstractTextualComponent.java','klimt/shape/TextBlockVertical.java']:
 path='src/main/java/net/sourceforge/plantuml/'+path;b=subprocess.check_output(['git','show',jr+':'+path],cwd=java);assert b==(java/path).read_bytes();extra.append(dict(path=path,revision=jr,sha256=sha(b)))
write(C/'additional-java-provenance.json',extra)
identity=dict(task_id='thread:delegated-task:command%3Amcp%3A3e6fe12c-b445-432a-9536-14b1ab9096f8%3Adelegate-task%3A432882cd-4556-4e95-862f-31b54103f709',model='gpt-6-astra',tool='T3 Code / Codex harness',independence='New checker; no maker involvement; no children; production unchanged',checkout=str(R),branch='review/sequence-pending-stripe-independent-20261105')
index=[]
for mechanism,info in findings.items():
 entries=[]
 for row in [x for x in fresh if x['mechanism']==mechanism]:
  p=Path(row['source']);dest=R/'test-diagrams/perturbations'/mechanism/p.name;dest.parent.mkdir(parents=True,exist_ok=True);assert not dest.exists();shutil.copy2(p,dest);shutil.copy2(p.with_suffix('.svg'),dest.with_suffix('.svg'))
  entries.append(dict(source=str(dest.relative_to(R)),golden=str(dest.with_suffix('.svg').relative_to(R)),axes=row['axes'],intended_valid=row['intended_valid'],observed_valid=row['observed_valid'],result='pass' if row['results']['after']['cli_comparison']['exit']==0 else 'fail',source_sha256=row['source_sha256'],reference_sha256=row['golden_sha256'],before_comparison_exit=row['results']['before']['cli_comparison']['exit'],comparison_exit=row['results']['after']['cli_comparison']['exit'],before_cli_exit=row['results']['before']['cli']['exit'],cli_exit=row['results']['after']['cli']['exit'],cli_stderr=row['results']['after']['cli']['stderr'],capture_set='followup10' if 'self_extra_' in p.name else 'fresh86',capture_index=row['index']))
 impl='d5478061ffbd2b1ec31a8ac513ff23b4489e3fa6' if mechanism=='sequence-single-pending-create' else '653b1ed7a49784e6e6205f3aebf69a16278f8d78'
 review=dict(schema_version=1,mechanism=mechanism,reviewer=identity,account_commit='4aaab169442b1e8a1d9e896d0cd3f019de6803d4',implementation_commit=impl,tested_production='653b1ed7a49784e6e6205f3aebf69a16278f8d78',evidence_tip='ae1e0de4d39840dd58074d89d40aa97304c6ca9f',before_revision='711dbca7c5e29592395f0927881aabfc2517b217',before_production='a4c3c35794ae1d7117d709134ed22c7191db9db8',java_revision=jr,commands=['uv run --no-project python '+str(C/'build_gates.py'),'uv run --no-project python '+str(C/'replay.py')+' docs/parity-reviews/sequence-pending-stripe-packet/maker-results/all-inputs.json replay2169','uv run --no-project python '+str(C/'replay.py')+' '+str(C/'fresh86.json')+' fresh86','uv run --no-project python '+str(C/'replay.py')+' '+str(C/'followup10.json')+' followup10','/Users/marcelo/rustuml-reference/bin/reference --cwd PRIVATE_CWD --timezone GMT+11:00 -- -pipe -tsvg (parent existing relay; exact substituted argv retained per result)'],perturbations=entries,findings=info['findings'],verdict=info['verdict'],next_obligations=info['next_obligations'],evidence_directory=str(D.relative_to(R)),counts=summary['fresh_mechanisms'][mechanism],limitations=['Whole failure rejects; bounded support never waives any failed counterexample or imports historical acceptance.','Stable broad gates ran once per exact revision before evidence registration; no repeated broad run for evidence-only files. Existing gates remain red.','Java argv derived from pinned wrapper, not process traced. Remote OpenJDK21.0.5/Graphviz12.2 historical font/Graphviz equivalence is unproved.','No combined correctness with concurrent creole/text_render/WBS packet.'])
 path=R/'docs/parity-reviews'/mechanism/'review.json';assert not path.exists();write(path,review);index.append(dict(mechanism=mechanism,review=str(path.relative_to(R)),verdict=info['verdict'],counts=review['counts']))
write(D/'review-index.json',index)
# Exact capture archive excludes only rebuildable binaries/targets; no reference or output is normalized.
archive_names=['replay2169','fresh86','followup10','welcome2','welcome-handoff','reference-inputs','self-followup','before','after']
with tarfile.open(D/'execution-captures.tar.gz','w:gz') as tar:
 for n in archive_names:tar.add(C/n,arcname=n)
archive_index={}
with tarfile.open(D/'execution-captures.tar.gz') as tar:
 for m in tar.getmembers():
  if m.isfile():archive_index[m.name]=sha(tar.extractfile(m).read())
write(D/'archive-files.json',archive_index)
for p in sorted(C.iterdir()):
 if not p.is_file():continue
 if p.name=='reference-results.json' or p.name=='followup-reference-results.json':continue # exact uncompressed bytes are present in gzip
 shutil.copy2(p,D/p.name)
write(D/'reviewer.json',identity)
# All nine imported rejected records retain their original verdict and exact bytes.
prior=read(R/'docs/parity-reviews/sequence-component-display-packet/checker-711-astra/review-index.json');assert len(prior)==9
write(D/'prior-verdicts.json',[dict(review=x['review'],verdict=read(R/x['review'])['verdict']) for x in prior]);assert all(read(R/x['review'])['verdict']=='rejected' for x in prior)
readme='''# Independent sequence pending CREATE / stripe review

**Packet rejected.** Tested production `653b1ed7a49784e6e6205f3aebf69a16278f8d78`, evidence tip `ae1e0de4d39840dd58074d89d40aa97304c6ca9f`; exact before packet `711dbca7c5e29592395f0927881aabfc2517b217` / production `a4c3c35794ae1d7117d709134ed22c7191db9db8`. New independent gpt-6-astra checker; evidence-only branch. No production fix or shared packet import.

## Disproof

Four fresh Java-valid outgoing external pending-CREATE sources were whole CLI matches before, and now panic at `crates/rustuml-render/src/sequence.rs:14287:81` (`Option::unwrap` on missing border target). Both border directions and both engines fail. Path: parser `validate_return_destinations` -> lowered `sequence_create_attachments` -> renderer `sequence_create_message_indices` -> `create_msg_idx` -> `created_inline` / `is_create_msg` -> `id_to_idx[msg.to].unwrap()`. The sole real participant is the source on an outgoing MessageExo. Java's `Step1MessageExo` / `CommunicationExoTile` do not use the ordinary creation-head path. Simply accepting ownership is not a complete render-model port.

All four sources and exact raw Java SVGs remain executable under `test-diagrams/perturbations/sequence-single-pending-create/`. The stable 2169 cohort did not expose these failures.

## Exact shipped CLI replay

| Cohort | Entries | Before match / valid-fail / invalid | After |
|---|---:|---:|---:|
| Preserved | 2083 | 1448 / 576 / 59 | 1448 / 576 / 59 |
| Checker | 64 | 42 / 16 / 6 | 50 / 8 / 6 |
| Maker | 20 | 0 / 16 / 4 | 16 / 0 / 4 |
| Shared handoff | 2 | 0 / 2 / 0 | 0 / 2 / 0 |
| Total | 2169 | 1490 / 610 / 69 | 1514 / 586 / 69 |

All maker CLI SVG hashes and all CLI/product-library/assisted-library status codes reproduce exactly. There are 2121 distinct source byte strings and 48 retained duplicates. The frozen cohort has 24 recoveries, zero lost passes in every mode, zero new valid parse errors and zero renderer panics. Invalid references remain in the denominator: Rust reject/accept changes 45/24 to 49/20. Two empty-reference `diff_one` comparison-helper panics are preserved separately, not relabeled renderer panics.

Parent1449 is not additive: 1123 overlapping paths and 326 unreplayed paths. The overlap has1097 distinct source byte strings; the326 paths contain286 distinct strings, of which278 are exclusive (8 duplicate overlapping bytes). No unreplayed path is credited.

## Fresh evidence and verdicts

The first86 and followup10 are distinct private relay batches:96 fresh entries,88 Java-valid and8 command-error references. All streams were hash-checked and XML/metadata/diagnostic/comparator-classified independently; zero timeouts or intention disagreements. Before:45 matches,43 valid failures,8 invalid. After:67 matches,21 valid failures (17 SVG differences plus4 renderer panics),8 invalid. There are26 recoveries and4 lost passes. No new Java-valid parse rejection; the four new runtime errors are renderer panics. No case is removed for failing.

| Mechanism | Fresh whole matches / valid entries | Verdict |
|---|---:|---|
| Single pending CREATE | 19/30 (+8 invalid retained) | Rejected |
| Queue stripe origin | 8/14 | Rejected |
| Message stripe baselines | 12/16 | Rejected |
| Self half-polygon order | 24/24 | Accepted, bounded to this matrix |
| Collections citation | 4/4 | Accepted, citation-only |

Queue failure is causal: the AST retains `dispatch`, while the unboxed/created queue paints zero stereotype labels instead of Java's two. Rust gates `stereo_ref` and height on `in_box`; Java `Participant.getDisplay` and `ComponentRoseQueue` pass the complete Display. All six missing-stereotype cases already failed before. Heading stripe controls likewise already fail: Java paints `Crest`/`Ridge`; Rust paints literal heading markers. Notes and dividers use separate Rust consumers, determined from source, and remain control failures. Neither old failure is attributed as a new regression, but neither is waived.

Both shared handoffs remain whole failures. All586 frozen valid failures remain listed in `remaining-failures.json`: reverse-created queue legacy; unclosed groups; mixed-kind Teoz boxes; multiline/nested boxes and englobers; ancestor-sum envelopes; tabs, URL Display, fonts, themes, shadows, notes, delays, chrome and factory cases. The TIM welcome handoff is separately replayed from exact historical commit `0f00130a26b2f300996d0bd427f9e70b4ef10e30`: two welcome-truncation references, both comparator-invalid/nonpositive and both revisions rendering Class. They are neither added to2169 nor called positive whole-output failures. No shared style/WBS combined correctness is claimed.

## Counts beyond whole matches

Real `--ast -pipe`, `-ttxt -pipe` and full SVG captures cover every2169 retained entry and every96 fresh entry. The maker148 subset reproduces4 ASCII parse rejections and144 unchanged. Across all2169, exactly4 AST and4 ASCII outcomes change (those invalids); across96 fresh,8 invalid rejections change, with88 valid AST/ASCII outputs unchanged. Four of those88 SVG renders panic, demonstrating why AST/ASCII success is not SVG evidence.

`semantic-inventory.json.gz` preserves Java/before/after tag counts, exact label multiplicities, available participant groups, text positions, polygon vertices and raw AST event counts. Inline created heads are not always grouped, so group counts alone are explicitly incomplete. The double-pending witness changes Roe labels3 to2; maker actor witness changes4 to3 while retaining3 CREATE events. All frozen valid increased-excess tag/label checks are empty. The full raw outputs are the evidence, not that diagnostic alone.

## Gates and provenance

Private release targets were built separately with jobs4. The golden submodule content was hydrated from the local clone at exact `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`; no local JVM ran. No setup/build failure occurred. Stable required gates ran ONCE per revision before evidence registration:

- Before parser574, renderer793; after parser575, renderer795 (12 ASCII), CLI13 in both.
- Both main library product tiers11181/11251, assisted10497/11251, sequence1411/1411. These are library diagnostics, not11251 CLI processes.
- Before standing744/1352; after794/1416,622 failures. Both guards4/6. All remain red; frozen inputs were not changed.
- Existing class-file workspace formatting, parser component dead-code Clippy and19 renderer Clippy findings reproduce. Exact failure-name snapshots were copied immediately per revision and are byte-identical.

All8462 baseline evidence paths and183 imported paths are byte-identical. All five accounts first appear in4aaab169 before their implementation commits, whose messages reference that account. Only three allowed sequence production files differ from baseline. No serialized field, parse/mod, ASCII, shared text/style/font, class, TIM, native, comparator, skip, baseline, allow-list or target changed. The collections function is unchanged after removing comments. No new fixture-keyed production branch or unexplained layout constant was found. All nine imported claims stay rejected.

Java source is pinned to71806a23780b04a5ccde2f8ceb5121edad5eb711 and hashed against the read-only clone; JAR SHAaa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053. All new Java references came through the parent's EXISTING relay, workersYD4ftmgc andrWaHIBFO, GMT+11:00. Java argv is wrapper-derived, not process-traced. Remote OpenJDK21.0.5 and Graphviz12.2 do not prove historical font/Graphviz equivalence.

## Reproduction and bounded obligations

`build_gates.py`, `replay.py`, `fresh_inputs.py`, `followup_inputs.py`, relay manifests, classification scripts, raw streams/status/provenance, exact commands and binary bindings are retained. `execution-captures.tar.gz` contains full before/after SVGs, ASTs, ASCII, comparator/library diagnostics and immediate gate snapshots; `archive-files.json` hashes every member. Absolute paths record the actual commands; remap paths to a new private cache for reuse. Do not replay generation through a new JVM/runner. The standing perturbation test discovers all new pairs recursively; no comparator or gate edit is required. Broad gates were not repeated after evidence-only registration.

Next work must separately model ordinary versus external creation consumers, complete queue Display/stereotype composition, and resolve heading/shared Display semantics while retaining the counterexamples. The two bounded acceptances do not accept the packet or transfer historical acceptance. All old reviews and accounts remain intact.
'''
(D/'README.md').write_text(readme)
write(D/'artifact-sha256.json',{str(p.relative_to(D)):sha(p.read_bytes()) for p in sorted(D.rglob('*')) if p.is_file() and p.name!='artifact-sha256.json'})
print('packaged',len(fresh),'fresh pairs',len(index),'reviews','archive bytes',(D/'execution-captures.tar.gz').stat().st_size)
