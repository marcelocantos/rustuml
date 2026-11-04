# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
R=Path.cwd();C=Path(__file__).parent;E=R/'docs/parity-reviews/sequence-consumer-contract-packet/checker-9784-astra-b4b6';E.mkdir(exist_ok=False)
sha=lambda b:hashlib.sha256(b).hexdigest()
def write(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
rows=json.loads((C/'fresh-pairs.json').read_text());before=json.loads((C/'fresh-before/measurements.json').read_text());after=json.loads((C/'fresh-after/measurements.json').read_text());summary=json.loads((C/'fresh-summary.json').read_text());audit=json.loads((C/'source-audit.json').read_text())
# Verify the relay input envelopes preserve both original bundle and exact manifest.
transport=json.loads((C/'transport-audit.json').read_text())
for p,t in zip([C,C/'followup'],transport):
 original=json.loads(gzip.decompress((p/'reference-input-bundle.json.gz').read_bytes()));uploaded=json.loads(gzip.decompress((p/'parent-relay-input.json.gz').read_bytes()));assert base64.b64decode(uploaded['manifest_base64'])==(p/'reference-manifest.json').read_bytes();assert uploaded['original_input_bundle_sha256']==sha((p/'reference-input-bundle.json.gz').read_bytes());assert all(uploaded[k]==original[k] for k in original);t['uploaded_original_bundle_fields_equal']=True;t['uploaded_exact_manifest_bytes_equal']=True
write(C/'transport-audit.json',transport)
canonical=[]
for row in rows:
 dest=R/'test-diagrams/perturbations'/row['mechanism'];dest.mkdir(exist_ok=True)
 src=Path(row['source']);source=dest/src.name;golden=source.with_suffix('.svg');stderr=source.with_suffix('.stderr')
 for old,new in [(src,source),(src.with_suffix('.svg'),golden),(src.with_suffix('.stderr'),stderr)]:
  assert not new.exists();new.write_bytes(old.read_bytes())
 assert sha(source.read_bytes())==row['source_sha256'];assert sha(golden.read_bytes())==row['stdout_sha256']
 canonical.append(dict(row,request_source=row['source'],source=str(source.relative_to(R)),golden=str(golden.relative_to(R)),stderr_path=str(stderr.relative_to(R))))
write(E/'fresh-pairs.json',canonical)
revs=dict(evidence_tip='9784e412146e3ece634d88ea0a1bddd6b42a82b6',before_evidence='bce6bea405d8b5efd55136f4768117115e7f634f',before_production='61e6aeed6eaf686b3497fbc696aadc03a64e69a2',after_production='d491d4bda47faf90d4751484d45aa1387fb1b290',java='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',golden_submodule='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a')
write(E/'revisions.json',revs)
mechanisms={'sequence-pattern2-whitespace':'df9078e2e162752328ffa52ebb28f227e56bc8f6','sequence-all-participant-display-consumers':'f4b3db075f61e2a5444777a305ab9bb64fd3685d','sequence-teoz-self-note-component':'47fb7bd3a27d62ed80cdca112721be7ca61fc2e6','sequence-note-over-command-domain':'d491d4bda47faf90d4751484d45aa1387fb1b290'}
findings={
 'sequence-pattern2-whitespace':[
 'The positive and negated class replacements exactly encode pinned Pattern2 ASCII TAB..CR, SPACE and NBSP. All 21 production sequence_regex call sites were inventoried; no raw Unicode \\s/\\S regex remains. No fixture-shaped predicate or fitted numeric constant was found.',
 'Whole-command equivalence is disproved: RegexLeaf#getFoxSignature + RegexConcat#match + FoxSignature prefilter require a whitespace bit; FoxSignature omits VT. Commands containing only VT separators can be refused by Java before the regex. Rust lacks this prefilter. These observed false accepts pre-exist the packet.',
 'Literal-space entry checks still bypass the shared regex in try_box, note-across and autonumber-resume. Fresh NBSP/FF/VT controls retain omissions. Non-regex trim/token and complete factory semantics were not ported; do not call this all-command parity.',
 'Source-map Java hashes are exact, but account CommandParticipant#getRegexConcat is nonexistent. Declaration grammars are in subclasses such as CommandParticipantA; the base class owns getRegexType/getOrderRegex. This is an account provenance defect, preserved without editing it.',
 'All 15 requested other Unicode spaces were probed in terminators and quoted participant names, both engines. Thirty terminators reject after, including 28 newly rejected; quoted labels retain independent font/metric mismatches. NBSP/VT/FF probes cover all requested command classes and 23 arrow variants. Negated colour-token probes keep original intent but six are Java-invalid because order follows terminal colour; they do not establish a whitespace discrepancy.',
 'Fresh 294: 179 matches / 63 SVG mismatches / 0 valid errors / 44 Java-invalid / 8 Java-export-error outcomes both revisions. Explicit fresh rejection contracts are retained separately. The two frozen message_gap false accepts follow regex refusal -> unconsumed=true -> parser.finish without rejection (T14.17). No negative is counted as an SVG match.',
 'Whole-output failures remain, so the mechanism is rejected despite bounded support for the class transformation.'],
 'sequence-all-participant-display-consumers':[
 'Pinned Participant#getDisplay filters stereotypes and preserves declaration-time order. The eight Rose component consumers compose Display and symbol in the reviewed order. Rust now shares its complete ordered Display painter and height, removes the boxed-only heuristic, and retains component-specific source-cited margins. No coordinate compensation or fixture-conditioned behavior was found.',
 'Account and adjacent comment cite nonexistent DrawableSetInitializer#createParticipant. The actual method is prepareParticipant, lines598-656; lines630-634 pass Participant#getDisplay to head/tail. This is a provenance defect even though the causal data flow is supported.',
 'Fresh 128 controls cover all eight kinds, rich long multiline aliases, unequal stereotype widths, colours, top/bottom, hidden stereotypes, footbox suppression, boxes/nested boxes, creation and destruction. Match counts rise47->119; no kind loses a prior positive match. Nine failures remain: nested Teoz boxes for all eight kinds and created control in legacy. They are preserved existing failures, not introduced packet regressions.',
 'All frozen CLI classifications are unchanged, sequence golden library1411/1411 remains green, and both main failure-name snapshots remain byte-identical; no golden loss from removal of the boxed-only heuristic was observed.',
 'Reject under the whole-output rule. Bounded Display improvements do not waive creation/box failures or complete spot/multiple/styled stereotype obligations.'],
 'sequence-teoz-self-note-component':[
 'CommunicationTileSelfNoteLeft/Right#getComponent explicitly use ComponentType.NOTE. Rust resolves returns then clones renderer-local events to select Note only for Teoz on-message left/right self-note consumers, before dimensions/scheduling/painting. The source AST remains untouched; no ordinate compensation was added.',
 'Fresh48 cases span rnote/hnote, left/right, multiline style, first/last owner, groups, return lowering, named over-self and ordinary nonself controls in both engines. Whole matches rise10->26 with no lost positive. All 48 source AST/YAML/ASCII captures remain byte-identical before/after; all legacy, named-over and nonself SVG controls remain byte-identical.',
 'Twenty-two whole-output failures persist in legacy self-note and nonself envelopes/ordinates. No new valid CLI error or SVG panic occurs. Reject because these remain executable counterexamples; bounded Teoz component-selection support is not whole-output acceptance.'],
 'sequence-note-over-command-domain':[
 'The mandatory participant domain is correct in Java, but the Rust implementation adds an unsafe post-match guard to a regex where both optional of and participant are optional.',
 'Introduced regression: alias of with note over of : text, coloured inline/multiline, rnote or hnote causes the optional of prefix to consume the identity greedily; the empty-capture check returns ParseError. Java requires PARTICIPANT and backtracks to capture of. All16 follow-up references are independently valid SEQUENCE SVGs;10 cases newly exit1 in Rust. Before those10 exited0 but mismatched SVG, so there are no lost whole matches despite new valid-input rejections.',
 'Fresh64:24 matches/32 mismatches/0 valid errors/8 Java-invalid before;24 matches/22 mismatches/10 valid errors/8 Java-invalid after. No lost rejection. The10 diff_one ParseError unwrap panics are library diagnostic failures; actual CLI exits1 cleanly with no SVG panic.',
 'Quoted/named/pair/alias/multiline/shape/colour/stereotype/URL and across alternatives remain tested. accross was deliberately taken from the factory regex; executeInternal explicitly rejects it. Immutable intended_valid=true is preserved, observed invalid recorded. Existing quoted/option/note-envelope failures remain separate from the10 introduced errors.',
 'Reject. Restore correct grammar backtracking for the of identity before a new independent review; preserve all failures and do not alter this account or historical evidence.']}
commands=[
 'cargo build --release -j 4 -p rustuml -p rustuml-oracle --example diff_one --bin rustuml (both exact production revisions; private CARGO_TARGET_DIR; commands.json)',
 'cargo test --release --lib -p rustuml-parser -p rustuml-render',
 'cargo test --release -p rustuml --test pipe_mode',
 'cargo test --release --no-fail-fast -p rustuml-oracle --test golden_no_oracle --test golden_pairs --test parity_perturbations --test harness_guards -- --nocapture',
 'uv run --no-project python docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py MANIFEST PRIVATE_BIN_DIR PRIVATE_OUTPUT_DIR (unchanged comparator; raw arrays and exact env in captures.tar.gz)',
 'uv run --no-project python docs/parity-reviews/sequence-consumer-contract-packet/maker-results/rejection_contract.py REPO_ROOT MANIFEST BEFORE_BIN AFTER_BIN OUTPUT_JSON',
 'Existing parent Monkey Man relay; exact per-case actual_wrapper_argv and actual_java_argv arrays retained in fresh-pairs.json and immutable reference-results envelopes; derived, not process-traced.'
]
for mech,implementation in mechanisms.items():
 pairs=[]
 for row,b,a in zip(canonical,before,after):
  if row['mechanism']!=mech:continue
  passed=row['observed_valid'] and a['cli']['exit']==0 and a['cli_comparison']['exit']==0
  pairs.append(dict(source=row['source'],golden=row['golden'],axes=row['axes'],source_sha256=row['source_sha256'],golden_sha256=row['stdout_sha256'],intended_valid=row['intended_valid'],observed_valid=row['observed_valid'],reference_outcome=row['reference_outcome'],before_cli_exit=b['cli']['exit'],after_cli_exit=a['cli']['exit'],before_comparator_exit=b['cli_comparison']['exit'],after_comparator_exit=a['cli_comparison']['exit'],result='pass' if passed else 'fail',note='Nonpositive reference outcomes are retained as failures, never SVG parity passes.' if not row['observed_valid'] else 'Whole actual-CLI comparison with unchanged comparator.'))
 review=dict(schema_version=1,mechanism=mech,reviewer=dict(task_id='thread:delegated-task:command%3Amcp%3A3e6fe12c-b445-432a-9536-14b1ab9096f8%3Adelegate-task%3Aaa41bca4-1418-4a0f-8f84-c25a34ef449d',model='gpt-6-astra, high reasoning, Codex harness in T3 Code',role='New independent checker; no prior involvement; no children or production edits'),account_commit='0b512262aaf4bcf0c0ab144b9f28c0719f329e4c',implementation_commit=implementation,tested_production_revision=revs['after_production'],baseline_production_revision=revs['before_production'],evidence_tip=revs['evidence_tip'],java_revision=revs['java'],jar_sha256=revs['jar_sha256'],commands=commands,perturbations=pairs,findings=findings[mech],counts=summary['mechanisms'][mech],evidence_packet=str(E.relative_to(R)),verdict='rejected',acceptance_rule='Any whole-output failure rejects; no inherited acceptance or bounded waiver.',residue=['OpenJDK21.0.5/Graphviz12.2 historical equivalence remains unproved.','Java argv is derived, not process-traced.','No combined correctness claim with concurrent creole fallback packet.','Broad gates ran once per exact production snapshot; fresh corpus compared independently without repeating broad gates.'])
 out=R/'docs/parity-reviews'/mech/'review.json';assert not out.exists();write(out,review)
# Copy concise standalone evidence; preserve all raw captures and command evidence in archive.
for name in ['packet-audit.json','source-audit.json','additional-java-provenance.json','transport-audit.json','replay-summary.json','fresh-summary.json','fresh-participant-kinds.json','gates-summary.json','standing-denominators.json','models-summary.json','rejections.json','fresh-rejections.json','classification-refinement.json']:
 shutil.copy2(C/name,E/name)
shutil.copy2(C/'independent/commands.json',E/'commands.json')
for label in ['before','after']:shutil.copy2(C/'independent'/(label+'-binding.json'),E/(label+'-binding.json'))
support=E/'support';support.mkdir()
for p in C.glob('*.py'):shutil.copy2(p,support/p.name)
for n in ['measure.py','compare_cli.rs']:
 shutil.copy2(R/'docs/parity-reviews/sequence-dependency-closure/checker-support'/n,support/n)
shutil.copy2(R/'docs/parity-reviews/sequence-consumer-contract-packet/maker-results/rejection_contract.py',support/'rejection_contract.py')
# Archive deliberately excludes build products and executable binaries.
selected=[]
for p in C.rglob('*'):
 if not p.is_file():continue
 rel=p.relative_to(C)
 if any(part.startswith('target-') or part.endswith('-bin') for part in rel.parts):continue
 selected.append((p,rel))
with tarfile.open(E/'captures.tar.gz','w:gz') as t:
 for p,rel in sorted(selected,key=lambda x:str(x[1])):t.add(p,arcname=str(rel),recursive=False)
write(E/'capture-inventory.json',dict(files=[dict(path=str(rel),bytes=p.stat().st_size,sha256=sha(p.read_bytes())) for p,rel in sorted(selected,key=lambda x:str(x[1]))],excluded='Cargo targets and binary executables; complete source/binary bindings preserved.'))
print('packaged',len(canonical),'pairs',len(selected),'captures',E)
