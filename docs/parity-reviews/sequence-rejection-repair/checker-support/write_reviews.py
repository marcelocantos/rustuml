# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Serialize this checker's source findings and measured verdicts; no production edits."""
import collections
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
OUT=Path(__file__).resolve().parent
CACHE=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-d382aba6')
CANDIDATE='0fb4e14b7227eb768d268128d42fa5f6d5a5b1af'
PRODUCTION='8291416539aa9c61589e4dd586ad65d3a135966c'
JAVA='71806a23780b04a5ccde2f8ceb5121edad5eb711'
fresh=json.loads((CACHE/'fresh/measurements.json').read_text())
replayed=json.loads((CACHE/'replay1728/measurements.json').read_text())
by_source={r['source']:r for r in replayed+fresh}
source_audit={r['mechanism']:r for r in json.loads((OUT/'source-audit.json').read_text())['sections']}
normalization={Path(r['path']).parent.name:r for r in json.loads((OUT/'normalization-audit.json').read_text())}
reviewer=dict(task_id='t3-d382aba6 independent sequence rejection-repair checker',model='gpt-6-astra',reasoning_effort='high',tool='Codex harness in T3 Code',independent_from_maker=True,children=0)
commands=[
 'git switch -c review/sequence-repair-d382aba6 0fb4e14b7227eb768d268128d42fa5f6d5a5b1af',
 'CARGO_TARGET_DIR='+str(CACHE/'target')+' CARGO_BUILD_JOBS=4 cargo build --release -p rustuml -p rustuml-oracle --bin rustuml --example diff_one',
 'Remote existing runner: /Users/marcelo/rustuml-reference/bin/reference --cwd /Users/marcelo/rustuml-reference/scratch/worker.NzRxjEfs --timezone GMT+11:00 -- -pipe -tsvg (one source on stdin per invocation; exact argv/status/streams in remote-captures.json.gz)',
 'python3 docs/parity-reviews/sequence-rejection-repair/checker-support/import_relay.py',
 'python3 docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py docs/parity-reviews/sequence-rejection-repair/checker-support/fresh-manifest.json '+str(CACHE/'final-bin')+' '+str(CACHE/'fresh'),
 'python3 docs/parity-reviews/sequence-rejection-repair/checker-support/checks.py',
 'python3 docs/parity-reviews/sequence-rejection-repair/checker-support/checks.py evidence',
 'python3 docs/parity-reviews/sequence-rejection-repair/checker-support/audit_evidence.py',
]
# These are adjudications, not an automated rule that converts numeric deltas to explanations.
findings={
 'sequence-return-configuration':('rejected','Three of six valid fresh cases match. Legacy depth1 repeated returns leave a 5px excess canvas; parallel return is dropped (40 Java elements versus 37 Rust), and nested return after destroy retains extra components. The orphan-note and repeated external return negatives correctly exit1. Unknown #notacolor777 is rejected by Java exit200 but accepted by Rust exit0. Color recognition, parallel ownership and destruction must feed the same ordinary-message model; optional color support alone is not closure.'),
 'sequence-reverse-self-component':('rejected','Four engine/order/async/attached-note controls match. Active B x-- B with an attached left note fails: Java begins its upper stroke at136.5649, Rust124.5649; Java32 elements versus Rust30. This confirms the missing reverse CROSSX decoration branch, not a font discrepancy. Port decoration endpoint adjustments before stroke/head drawing and share them with constraints and note wrappers.'),
 'sequence-preferred-component-flow':('rejected','Four folded-note/ordinary-participant engine/shadow/padding controls match. Actor/database shadow control fails: canvas151px Java versus154px Rust, centers shift1.5px and actor label y differs3px. Preferred versus painted dimensions must come from each participant component; a uniform shadow increment cannot represent all kinds. Attached right-note shadow failures also bound the purported shared note flow.'),
 'sequence-page-wrapper-hierarchy':('rejected','All six fresh cases fail. In combined wrappers, valid legend left is absent from Rust (41 versus38 elements,262px versus211px legacy). Top legend is painted in the wrong wrapper position. Styled caption FontSize20/Margin6/Padding3 produces197px height versus167px. Both newpage engines fail. Use the parsed full legend placement and resolved styled TextBlocks through the wrapper chain; do not compensate by fixed page offsets.'),
 'sequence-teoz-event-history':('rejected','Four active grouped external border/direction cases match. Parallel message ownership loses the message/activation (35 versus30 elements). Create/run/destroy differs in lifeline start and canvas height218px Java versus228px Rust. All four initial order/create matrix inputs are Java-negative because deactivate/activate are attached to the same message; Rust accepts all four. They are retained as rejection evidence, never valid history passes. Port message-parallel identity, CREATE and standalone DESTROY tiles and Java input validation; valid coverage comes from the six remaining probes.'),
 'sequence-teoz-callback-lifecycle':('rejected','Five depth/group-note/delay callback controls match, supporting the ordinary three-pass traversal and staircase repair. Standalone destruction after a group then a self arrow fails: Java266px versus248px, the missing18px cross tile propagates to subsequent events and lifelines. Callback replay cannot be accepted separately from the scheduler that supplies its y values. Preserve lifecycle state across measurement/background, with creation and destroy metadata, rather than adding a fitted final offset.'),
 'sequence-teoz-group-tile-composition':('rejected','Four order/nesting/reversed-span/folded-note controls match. Hexagonal note inside a group fails: group/participant x positions differ by0.4016px despite the same source text widths. Java routes every note style through NoteTile bounds; Rust shared geometry returns None for non-folded notes and falls back to older reconstruction. Extend component preferred/allocation/bound dataflow to note styles, retaining the initial-versus-final Real phase.'),
 'sequence-teoz-note-component-flow':('rejected','Both left attachments match; both right attachments with padding3/shadow and opposite self directions fail. Java canvas390/328px versus Rust393/331px while inner elements otherwise compare equal, identifying residual envelope propagation rather than general text metrics. Audit right-wrapper preferred width versus painted shadow and canvas maximum. Source inspection also finds CommunicationTileNoteRight creation-specific posD branch absent from the ordinary-right helper; that branch is source-only residue, not an invented fresh test.'),
 'sequence-teoz-real-bound-capture':('rejected','All five direct folded-note tests match across asymmetric head order, later wide arrows and an earlier englober. This supports the captured initial-width/live-center mechanism. It does not establish the unqualified NoteTile invariant: the cross-linked fresh hexagonal group case fails and bypasses the shared geometry phase helper, while Java NoteTile supplies bounds for all styles. Reject the broad mechanism claim; the narrow folded-note phase algebra is supported, not disproved. Preserve that algebra while completing other note component paths.'),
 'sequence-delay-label-allocation':('rejected','Four asymmetric whitespace/engine/padding controls match. The fresh multiline grouped delay fails: one text element is lost and canvas259px Java versus246px Rust. Java LABEL becomes Display.getWithNewlines and preferred TextBlock dimensions; Rust still measures/paints one scalar label. Whitespace preservation is established for one line, but allocation and text flow do not generalize to the claimed component.'),
 'sequence-box-title-grammar':('accepted','All four quoted/unquoted, colored/trailing-space, legacy/Teoz inputs match. Source NAME2 capture and Rust unquoted-title storage agree, with original account chronology verified. Acceptance is restricted to title tokenization and retained title text before component measurement. Nested englober ownership, style and margins are separate downstream mechanisms and remain rejected; this is not acceptance of the sequence repair.'),
 'sequence-teoz-englober-margins':('rejected','All four count/nesting/boxPadding/hide-footbox/footer controls fail. Even flat one-participant box is317x158 Java versus296x154 Rust, with frame x23 versus16, exactly the chosen boxPadding7 missing from the outer placement. Nested cases additionally lose parent title-height and containment: inner title is drawn at outer y instead of below it. Model ancestor englobers and outer margins before final frame/canvas composition; no coordinate tuning is justified.'),
}

def entry(r,axes=None):
 code=r['cli_comparison']['exit']
 out={k:r[k] for k in ['source','golden','source_sha256','golden_sha256']}
 out.update(axes=axes or r.get('axes',['retained source topology','retained label/style variation']),result='pass' if code==0 else 'fail',classification={0:'valid-SVG-match',1:'valid-SVG-mismatch',2:'negative-reference',3:'comparison-error'}[code],cli_exit=r['cli']['exit'],comparator_exit=code,diagnostics='docs/parity-reviews/sequence-rejection-repair/checker-support/'+('fresh.json.gz' if r in fresh else 'replay1728.json.gz'))
 if r in fresh:
  provenance=json.loads((ROOT/r['source']).with_suffix('.provenance.json').read_text())
  out.update(java_exit=provenance['exit_code'],reference_environment='docs/parity-reviews/sequence-rejection-repair/checker-support/remote-environment.json')
  if code==2:out['negative_behavior']='correct CLI rejection; not a positive SVG pass' if r['cli']['exit'] else 'Rust accepts Java-negative input'
 return out

common=dict(schema_version=1,reviewer=reviewer,implementation_commit=PRODUCTION,tested_candidate_revision=CANDIDATE,java_revision=JAVA,java_jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',commands=commands,input_frontend=dict(fresh='raw -pipe -tsvg on stdin through pinned remote wrapper, GMT+11:00',transport='temporary T3 relay, exact input and result bundle hashes verified',preserved='original reference bytes and per-reference provenance unchanged',local_java=False),evidence='docs/parity-reviews/sequence-rejection-repair/checker-support/README.md')
records=[]
for mechanism,(verdict,finding) in findings.items():
 audit=source_audit[mechanism]
 rows=[r for r in fresh if r['cohort']==mechanism]
 record=dict(common,mechanism=mechanism,account_commit=normalization[mechanism]['original_commit'],account_normalization_commit='89cac979e2053cc07fe791e28f9f31bfc125fdb5',perturbations=[entry(r) for r in rows],source_audit=audit,findings=[audit['source_supported'],finding,'No new fixture label predicates, golden reads, branch-count constant ladders or coordinate string surgery found. See source-audit.json for Java provenance and semantic constant reuse concerns. Every valid failure is binding; no dependency waiver.'],verdict=verdict)
 if mechanism=='sequence-teoz-real-bound-capture':record['fresh_cross_mechanism_evidence']=[entry(r) for r in fresh if r['source'].endswith('checker_d382_hex_note.puml')]
 path=ROOT/'docs/parity-reviews'/mechanism/'review.json';assert not path.exists();path.write_text(json.dumps(record,indent=2)+'\n');records.append(str(path.relative_to(ROOT)))

# Re-evaluate the old claims in new files. Original verdicts and bytes remain intact.
links={
 'sequence-component-padding-flow':['sequence-preferred-component-flow','sequence-delay-label-allocation','sequence-teoz-note-component-flow'],
 'sequence-external-arrow-grammar':['sequence-teoz-event-history'],
 'sequence-note-component-envelope':['sequence-preferred-component-flow','sequence-teoz-note-component-flow'],
 'sequence-page-textblock-composition':['sequence-page-wrapper-hierarchy'],
 'sequence-participant-shadow-envelope':['sequence-preferred-component-flow'],
 'sequence-recursive-group-constraints':['sequence-preferred-component-flow','sequence-teoz-group-tile-composition'],
 'sequence-return-live-message-model':['sequence-return-configuration'],
 'sequence-self-arrow-async-stroke':['sequence-reverse-self-component'],
 'sequence-self-arrow-attached-lifecycle':['sequence-reverse-self-component','sequence-teoz-event-history','sequence-teoz-callback-lifecycle'],
 'sequence-self-arrow-envelope':['sequence-reverse-self-component','sequence-page-wrapper-hierarchy'],
 'sequence-self-return-message-model':['sequence-return-configuration','sequence-reverse-self-component'],
 'sequence-teoz-life-event-bounds':['sequence-teoz-event-history','sequence-teoz-callback-lifecycle'],
 'sequence-teoz-note-tile-bounds':['sequence-teoz-note-component-flow','sequence-teoz-group-tile-composition'],
 'sequence-teoz-self-tile-history':['sequence-teoz-event-history','sequence-teoz-callback-lifecycle','sequence-reverse-self-component'],
}
for name in json.loads((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/evidence/review-index.json').read_text()):
 old=json.loads((ROOT/name).read_text());mechanism=old['mechanism']
 primary=[entry(by_source[p['source']],p['axes']) for p in old['perturbations']]
 relevant=[r for r in fresh if r['cohort'] in links[mechanism]]
 verdict='rejected'
 extra='The original primary valid inputs now match. New cross-mechanism controls exercise the changed consumers on at least two independent axes; linked valid failures remain binding to whole-source closure. This does not imply every isolated equation is wrong.'
 if mechanism=='sequence-external-arrow-grammar':
  relevant=[r for r in relevant if 'checker_d382_external_' in r['source']]
  verdict='accepted'
  extra='All eight retained primary cases and all four fresh active/grouped border-side versus direction cases match. Boundary normalization and ownership are accepted for these external-arrow cases. This does not accept parallel events, arbitrary return colors or creation/destruction history; no such generalization is claimed.'
 elif mechanism=='sequence-recursive-group-constraints':
  extra='Retained legacy competing-member controls now match and fresh legacy padding/shadow/span controls also match. No new direct disproof of isolated legacy raw-minimum algebra is claimed. Full dependency acceptance remains withheld: the original41 font controls are still unresolved, and the recovery includes shared group/component paths with fresh note-style failures. Teoz hex-note failure is explicitly not represented as a counterexample to the legacy-only raw-minimum formula.'
 record=dict(common,mechanism=mechanism,account_commit=old['account_commit'],previous_review=name,previous_review_commit='704b80d0c31def61e0ccd6ff78565b017fd675a0',previous_implementation_commit=old['implementation_commit'],perturbations=primary,fresh_cross_mechanism_evidence=[entry(r) for r in relevant],findings=[extra]+[findings[m][1] for m in links[mechanism] if mechanism!='sequence-external-arrow-grammar']+['Historic review.json and every retained input/reference are unchanged. New record describes candidate0fb4e14b only. Cross-linked sources are counted once in the68-case fresh matrix, not copied into additional directories.'],verdict=verdict)
 path=(ROOT/name).with_name('review-2.json');assert not path.exists();path.write_text(json.dumps(record,indent=2)+'\n');records.append(str(path.relative_to(ROOT)))
(OUT/'review-index.json').write_text(json.dumps(records,indent=2)+'\n')
summary=dict(total=68,counts=dict(collections.Counter(r['cli_comparison']['exit'] for r in fresh)),java_exit=dict(collections.Counter(json.loads((ROOT/r['source']).with_suffix('.provenance.json').read_text())['exit_code'] for r in fresh)),mechanisms={m:dict(verdict=v,counts=dict(collections.Counter(r['cli_comparison']['exit'] for r in fresh if r['cohort']==m))) for m,(v,_) in findings.items()},negative_cases=[entry(r) for r in fresh if r['cli_comparison']['exit']==2],fresh_before_claim=False)
(OUT/'fresh-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print('Wrote12new and14successor records; original records unchanged.')
