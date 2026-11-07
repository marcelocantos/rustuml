# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,re,shutil,subprocess,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();BASE='dc438fd0cd0f5edd44816db81f0d765bf48ce4e6';REVIEW='0fb379a26ab64ead0744278935a0b6c64082ef35';D=C/'durable-package'
D.mkdir(exist_ok=False)
def read(p):
 p=Path(p);return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_text())
def write(n,v):
 p=D/n;p.parent.mkdir(parents=True,exist_ok=True);b=(json.dumps(v,indent=2)+'\n').encode();p.write_bytes(gzip.compress(b,mtime=0) if p.suffix=='.gz' else b)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],text=True).strip()
DEST=Path('docs/parity-reviews/sequence-command-admission-packet/maker-results');mapping={}
for folder in ['reference-inputs','size-inputs']:
 for p in sorted((C/folder).iterdir()):
  q=D/'inputs'/p.name;q.parent.mkdir(exist_ok=True);shutil.copy2(p,q);mapping[str(p)]=str(DEST/'inputs'/p.name)
manifest=read(C/'all3569.json')
for row in manifest:
 for k in ['source','golden']:row[k]=mapping.get(row[k],row[k])
write('all-inputs.json',manifest);write('capture-path-mapping.json',mapping)
for old,new in [('before3569.json','before.json.gz'),('after3569.json','after.json.gz')]:
 rows=read(C/old)
 for row in rows:
  for k in ['source','golden']:row[k]=mapping.get(row[k],row[k])
 write(new,rows)
for n in ['summary.json','rejection-contract.json','changes.json','remaining-failures.json','parent-approved-exceptions.json','dispatch-handoff.json','cohort-overlap.json','model-summary.json','model-changes.json','successful-model-diffs.json','family-changes.json','golden-byte-changes.json','golden-new-rejections.json','unimplemented-command-residue.json','final-source-identity.json','main-failure-name-comparison.json','standing-failures.json','reference-classification.json','size-reference-classification.json','final-checks.json','before-binding.json','final-binding.json','command-inventory-raw.json','inventory.json']:
 shutil.copy2(C/n,D/n)
for n in ['size-diagnostics.json','before-golden-cli/comparisons.json','final-golden-cli/comparisons.json']:
 write(n.replace('/','-')+'.gz',read(C/n))
for p in C.glob('*.py'):shutil.copy2(p,D/p.name)
# Preserve raw relay inputs, results, manifests and transport validation unchanged.
for prefix in ['', 'probe-', 'size-']:
 for name in ['reference-results.json','reference-results.json.gz','reference-transport.json','reference-manifest.json','reference-input-bundle.json.gz','parent-relay-input.json.gz','parent-upload-validation.json','parent-relay-validation.json']:
  p=C/(prefix+name)
  if p.exists():shutil.copy2(p,D/p.name)
shutil.copy2(C/'SequenceCommandsProbe.java',D/'SequenceCommandsProbe.java')
for folder in ['final-logs','final-failure-names']:
 for p in (C/folder).iterdir():
  q=D/'logs'/folder/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
for p in C.glob('*.log'):
 q=D/'logs'/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
old=git('ls-tree','-r','--name-only',BASE,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();changed=set(git('diff','--name-only',BASE,'HEAD').splitlines());assert not set(old)&changed
imported=git('diff-tree','--no-commit-id','--name-only','-r',REVIEW).splitlines()
# Index tree objects, avoiding one subprocess per file.
def tree(rev):
 return {x.split('\t',1)[1]:x.split()[2] for x in git('ls-tree','-r',rev).splitlines()}
t0=tree(REVIEW);t1=tree('HEAD');assert all(t0[p]==t1[p] for p in imported)
modified=git('diff','--diff-filter=M','--name-only','193480c26','HEAD').splitlines();assert set(modified)=={'crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs'}
write('protected-evidence.json',dict(original_rejection=REVIEW,import_commit=git('rev-parse','193480c26'),old_protected_paths=len(old),imported_paths=len(imported),byte_unchanged=True,modified_existing_files=modified,model_ascii_factory_shared_files_unchanged=True,comparator_sha256=sha(R/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs'),measure_sha256=sha(R/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py')))
mechanisms=[('sequence-registered-command-rejection','3b38384b3',['ebbbe28c0']),('sequence-command-color-execution','3b38384b3',['d10628bd7','268fa9f8b']),('sequence-footbox-command-consumption','3b38384b3',['2cc081521']),('sequence-local-attribute-serialization','3b38384b3',['4ac3ccba2']),('sequence-resume-citation-correction','3b38384b3',['2cc081521']),('sequence-command-size-admission','8efd66082',['ba3d0196b']),('sequence-command-size-dispatch-correction','297c7a1ea',['c525fd32a'])]
write('implementation-map.json',dict(production=git('rev-parse','HEAD'),mechanisms=[dict(mechanism=m,account_commit=git('rev-parse',ac),implementation_commits=[git('rev-parse',x) for x in im],status='reverted; not part of final behavior' if m=='sequence-command-size-admission' else 'requires NEW independent checker') for m,ac,im in mechanisms],files=modified,ownership='Sequence parse/render only. No model, ASCII, parse/mod, shared style/text/font, other-family, target, harness or baseline edits.',no_acceptance=True))
write('citation-corrections.json',dict(corrections=[dict(original='CommandAutonumberResume#getRegexConcat',actual='CommandAutonumberResume#getConcat',source='sequencediagram/command/CommandAutonumberResume.java',account='sequence-resume-citation-correction'),dict(original='PSystemBuilder#initFactories in the new size-dispatch account',actual='PSystemBuilder private constructor, lines128–132',source='PSystemBuilder.java',note='New evidence corrects method naming without rewriting the committed account; factory order and createPSystem230–255 remain the causal path.')],java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711'))
shared=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45/docs/parity-reviews/shared-svg-attributes')
for name in ['sequence-handoff.json','java-serialization-trace.json']:
 q=D/'source-handoffs'/name;q.parent.mkdir(exist_ok=True);shutil.copy2(shared/name,q)
write('source-handoffs/hashes.json',{n:sha(shared/n) for n in ['sequence-handoff.json','java-serialization-trace.json']})
write('intermediate-runs.json',dict(candidate1='candidate1/new-valid-errors.json in captures.tar.gz; all15 preserved and later exactly parent-approved; no positive pass lost.',candidate2='All5082 inputs captured (3533 retained +28 maker +1521 sequence goldens), source/binary binding and output preserved.',initial_unit_failures=['registry compiler lowercase named-group syntax, registry-unit-first.log and full draft source','legacy note-on-link test asserted an unregistered sequence command; corrected to reject, full second log preserved','legacy error-string expectation retained No such color','footbox routing initially shadowed by permissive message handler; initial log and corrected routing preserved'],size_guard=dict(account=git('rev-parse','8efd66082'),implementation=git('rev-parse','ba3d0196b'),correction_account=git('rev-parse','297c7a1ea'),revert=git('rev-parse','c525fd32a'),introduced_valid_errors=4,retained_final_valid_errors=0,source_equivalence='final-source-identity.json',controls='size-diagnostics.json.gz'),broad_runs=1,broad_revision=git('rev-parse','268fa9f8b'),reason_not_repeated='Exact reversal restored all production/build/test source bytes. Broad gates remain bound to the original tested revision, with explicit identity proof.'))
remaining=read(C/'remaining-failures.json');contract=read(C/'rejection-contract.json')
write('dependencies.json',dict(acceptance='NOT accepted; all mechanisms need fresh independent checking. No partial metrics substitute for full output.',full_remaining_failures='remaining-failures.json',valid_mismatches=866,valid_cli_errors=17,parent_approved_new_errors=15,previous_valid_errors=[x for x in remaining if x['after_exit']!=0 and x['before_exit']!=0 and x['after_status']!=2],nonpositive=241,dependencies=[dict(target='T14.21',cause='Factory syntax-error fall-through requires family admission contracts;13 approved misroutes remain errors, four2001-unit controls retain wrong Sequence family. No new exception for size controls.'),dict(target='T14.22',cause='Two exact retained TIM line-continuation sources produce invalid evaluated sequence text; parent-approved new errors, no preprocessor edits.'),dict(target='T14.20',cause='20 retained Java exporter-failure references,8historical+12checker, stay nonpositive; Rust does not reproduce Java crash.'),dict(target='T14.13',cause='Shared heading/stripe semantics; headings, notes/dividers remain failures.'),dict(target='T14.11',cause='Logical font mapping/metrics remain unowned failures.'),dict(cause='Registered but unimplemented command classes/variants and partial color style materialization; full command inventory plus actual warnings in unimplemented-command-residue.json.'),dict(cause='Note geometry/envelopes, nested Teoz boxes, quoted activation, shaped-across, short external ? arrows, participant/stereotype and page controls remain in retained failures.'),dict(cause='TIM welcome/factory and other-family historical false successes remain;12 historical nonpositive successes are named in rejection-contract.json.')],size_controls='8 Java-valid;4SEQUENCE/4CLASS;2match6mismatch final, all sources and SVGs unchanged',new_capture_classification='28batch:18valid10invalid. Size8:8valid, including4intended-invalid that renderCLASS. Intention disagreements preserved.'))
# Archive all private captures/scripts/snapshots, excluding executable binaries and this staging directory.
files=[]
for p in sorted(C.rglob('*')):
 rel=p.relative_to(C)
 if not p.is_file() or rel.parts[0]=='durable-package' or any(x.endswith('-bin') for x in rel.parts) or p.name in ['rustuml','diff_one','compare_cli']:continue
 files.append((p,rel))
write('capture-inventory.json',dict(files=[dict(path=str(r),bytes=p.stat().st_size,sha256=sha(p)) for p,r in files],excluded='Executable binaries only, with SHA256 bindings retained. Sources/references/stdout/stderr/status/model/YAML/ASCII and every intermediate failure are archived.'))
with tarfile.open(D/'captures.tar.gz','w:gz') as tar:
 for p,rel in files:tar.add(p,arcname=str(rel),recursive=False)
print('staged',len(files),'capture files', (D/'captures.tar.gz').stat().st_size,'archive bytes',flush=True)
