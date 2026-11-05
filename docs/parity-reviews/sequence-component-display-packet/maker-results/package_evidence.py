# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package verified private maker captures. Does not execute Java or comparators."""
import collections,gzip,hashlib,json,re,shutil,subprocess,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/sequence-component-display-packet/maker-results';BASE='672cd8f26e1d7c6725d8971e01ad7575defd8567'
MODES=['cli_comparison','product_library','assisted_library']
def read(p):return json.loads(p.read_text())
def sha(b):return hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],text=True).strip()
def write(n,v):
 p=D/n;p.parent.mkdir(parents=True,exist_ok=True);b=(json.dumps(v,indent=2)+'\n').encode();p.write_bytes(gzip.compress(b,mtime=0) if n.endswith('.gz') else b)
def stats(rows):
 return dict(entries=len(rows),matches=sum(r['cli_comparison']['exit']==0 for r in rows),valid_failures=sum(r['cli_comparison']['exit']==1 for r in rows),invalid=sum(r['cli_comparison']['exit']==2 for r in rows),invalid_cli_exits=dict(collections.Counter(r['cli']['exit'] for r in rows if r['cli_comparison']['exit']==2)),valid_cli_errors=[r['source'] for r in rows if r['cli_comparison']['exit']!=2 and r['cli']['exit']!=0],cli_panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']],render_panics=[r['source'] for r in rows if r['library_render_exit']==101],mode_exits={m:dict(collections.Counter(r[m]['exit'] for r in rows)) for m in MODES},diagnostic_panics={m:[r['source'] for r in rows if r[m]['exit']==101] for m in MODES[1:]},cli_equals_library=sum(r['cli_equals_library'] for r in rows))
D.mkdir(parents=True,exist_ok=True)
before=read(C/'before2063/measurements.json')+read(C/'before20/measurements.json');after=read(C/'final2083/measurements.json');assert len(before)==len(after)==2083
changes=[]
for old,new in zip(before,after):
 assert old['source']==new['source']
 for k in ['source','golden']:assert old[k+'_sha256']==new[k+'_sha256']==sha((R/new[k]).read_bytes())
 changes.append(dict(source=new['source'],golden=new['golden'],source_sha256=new['source_sha256'],golden_sha256=new['golden_sha256'],before_comparator=old['cli_comparison']['exit'],after_comparator=new['cli_comparison']['exit'],before_cli=old['cli']['exit'],after_cli=new['cli']['exit'],before_svg_sha256=old['cli']['svg_sha256'],after_svg_sha256=new['cli']['svg_sha256']))
losses={m:[b['source'] for a,b in zip(before,after) if a[m]['exit']==0 and b[m]['exit']!=0] for m in MODES}
assert not any(losses.values());assert not stats(after)['valid_cli_errors'];assert not stats(after)['cli_panics'];assert not stats(after)['render_panics']
for n,v in [('before-measurements.json.gz',before),('measurements.json.gz',after),('before-after.json.gz',changes),('remaining-failures.json.gz',[r for r in after if r['cli_comparison']['exit']!=0]),('negative-status.json',[r for r in changes if r['after_comparator']==2])]:write(n,v)
manifest=[]
for row in after:
 item={k:row[k] for k in ['source','golden','cohort']}
 for k in ['source','golden']:
  p=Path(row[k])
  if p.is_absolute():
   q=D/'inputs'/p.name;q.parent.mkdir(exist_ok=True);shutil.copy2(p,q);item[k]=str(q.relative_to(R))
 manifest.append(item)
write('all-inputs.json',manifest)
for p in (C/'reference-inputs').glob('*.stderr'):shutil.copy2(p,D/'inputs'/p.name)
for n in ['reference-results.json.gz','reference-transport.json','reference-manifest.json','reference-input-bundle.json.gz','reference-classification.json','parent-upload-validation.json','parent-relay-validation.json','before-binding.json','stable-binaries.json','final-binaries.json','first-broad-delta.json','complete-source.json','complete-binaries.json','member-binaries.json']:
 shutil.copy2(C/n,D/n)
for p in C.glob('*.py'):shutil.copy2(p,D/p.name)
for folder in ['stable-logs','final-logs','first-broad-failure-names']:
 for p in (C/folder).iterdir():
  q=D/'logs'/folder/(p.name if p.suffix=='.gz' else p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(p.read_bytes() if p.suffix=='.gz' else gzip.compress(p.read_bytes(),mtime=0))
shutil.copytree(C/'dependency-handoff',D/'dependency-handoff',dirs_exist_ok=True)
# Preserve every intermediate/source/provenance artefact; binaries remain private.
with tarfile.open(D/'captures.tar.gz','w:gz') as t:
 for p in sorted(C.rglob('*')):
  rel=p.relative_to(C)
  if p.is_file() and not rel.parts[0].endswith('-bin'):t.add(p,arcname=str(rel))
binding=read(C/'final-binaries.json');assert binding['revision']==git('rev-parse','HEAD')
for p,h in binding['source_files'].items():assert sha((R/p).read_bytes())==h
for folder,record in [('final-bin',binding),('before-bin',read(C/'before-binding.json'))]:
 for n,h in record['binaries'].items():assert sha((C/folder/n).read_bytes())==h
write('binaries.json',dict(before=read(C/'before-binding.json'),before_revision=BASE,after=binding,binding='Before binaries retain prior clean b4f37ab5 source binding, byte-identical production to baseline672cd8f. Final binaries built from clean a4c3c357. Intermediate display-bin captures retained without a complete simultaneous source snapshot; no exact-source revision claim for that draft.'))
write('checks.json',read(C/'final-checks.json'))
log=(C/'final-logs/main-tiers-and-guards.log').read_text();prod=re.search(r'golden_no_oracle: (\d+)/(\d+)',log);ass=re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped',log);seq=re.search(r'^  sequence\s+(\d+)/(\d+)',log,re.M);assert prod and ass and seq
main_changes={}
for n in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 old=gzip.decompress((R/'docs/parity-reviews/sequence-composed-components-packet/maker-results/logs'/(n+'.gz')).read_bytes());new=(R/'test-diagrams'/n).read_bytes();main_changes[n]=dict(recovered=sorted(set(old.decode().splitlines())-set(new.decode().splitlines())),newly_failing=sorted(set(new.decode().splitlines())-set(old.decode().splitlines())),sha256=sha(new));(D/'logs'/(n+'.gz')).write_bytes(gzip.compress(new,mtime=0))
assert all(not x['newly_failing'] for x in main_changes.values())
units=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/units.log').read_text());cli=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/cli.log').read_text())
section=log[log.index('held-out no-oracle perturbations diverged'):];fails=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',section,re.M));registered=len(list((R/'test-diagrams/perturbations').rglob('*.puml')))
cohorts={n:dict(before=stats(before[a:b]),after=stats(after[a:b])) for n,a,b in [('preserved2021',0,2021),('checker42',2021,2063),('maker20',2063,2083)]}
summary=dict(kind='Maker evidence only; new independent checker required',baseline=BASE,implementation=git('rev-parse','HEAD'),before=stats(before),after=stats(after),cohorts=cohorts,gains=[r for r in changes if r['before_comparator']==1 and r['after_comparator']==0],losses_by_mode=losses,main_library_tiers=dict(product_pass=int(prod[1]),eligible=int(prod[2]),assisted_pass=int(ass[2]),assisted_failures=int(ass[3]),skipped=int(ass[4]),sequence=f'{seq[1]}/{seq[2]}',changes=main_changes,label='Library APIs, not 11251 CLI subprocesses'),units=dict(parser=int(units[0]),renderer=int(units[1]),cli_pipe=int(cli[0]),ascii_included=True),guards='4/6',standing_perturbations=dict(entries=registered,failures=len(fails),matches=registered-len(fails),cwd=str(R/'crates/rustuml-oracle')),repeat_reason='cf3eafb2 first broad run lost sequence/seq_box_with_queue:11180 product,10496 assisted,1410sequence. c0ae326f account precedes a4c3c357 member-composition correction. All68 valid main queue controls pass. Code change justifies final broad rerun. Every first failure is retained.')
write('summary.json',summary)
overlap={}
for n,a,b in [('preserved2021',0,2021),('checker42',2021,2063),('maker20',2063,2083)]:
 rows=after[a:b];overlap[n]=dict(entries=len(rows),unique_sources=len({r['source_sha256'] for r in rows}),overlap_prior_bytes=len({r['source_sha256'] for r in rows}&{r['source_sha256'] for r in after[:a]}))
write('cohort-overlap.json',dict(entries=len(after),unique_paths=len({r['source'] for r in after}),unique_source_bytes=len({r['source_sha256'] for r in after}),duplicate_entries=len(after)-len({r['source_sha256'] for r in after}),cohorts=overlap,parent1449=dict(overlap_paths=1123,parent_only_paths_not_replayed=326,overlap_unique_bytes=1097,parent_only_unique_bytes=278,provenance='checker-672-astra denominator evidence; parent cohort overlaps and is not additive'),note='Every duplicate is measured. Invalid references remain separate; no positive parity credit. No unrelated parent growth imported.'))
old=git('ls-tree','-r','--name-only',BASE,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();changed=set(git('diff','--name-only',BASE,'HEAD').splitlines());assert not set(old)&changed
imp=git('diff-tree','--no-commit-id','--name-only','-r','4dd330b6').splitlines();assert not set(imp)&set(git('diff','--name-only','4dd330b6','HEAD').splitlines())
modified=git('diff','--diff-filter=M','--name-only','16b28fc6','HEAD').splitlines();allowed=['crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs'];assert set(modified)<=set(allowed)
write('protected-evidence.json',dict(import_commit=git('rev-parse','4dd330b6'),local_import=git('rev-parse','16b28fc6'),prior_evidence_paths=len(old),imported_paths=len(imp),prior_and_imported_byte_unchanged=True,modified_existing_files=modified,ascii='No ASCII file changes in this packet; existing constructor authorizations not expanded.',comparator_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),measure_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes())))
accounts=[('sequence-dressing-owned-parts','bf400d84',['83a49613']),('sequence-created-image-event-ownership','bf400d84',['cf3eafb2']),('sequence-complete-participant-display','bf400d84',['cf3eafb2']),('sequence-empty-rule-title-atom','bf400d84',['b653a5f8']),('sequence-unclosed-note-command','bf400d84',['71422f1b']),('sequence-self-part-consumer','c31c4216',['cf3eafb2']),('sequence-participant-kind-font-consumer','9e6a899c',['cf3eafb2']),('sequence-head-englober-component-height','9e6a899c',['cf3eafb2']),('sequence-englober-member-composition','c0ae326f',['a4c3c357'])]
write('implementation-map.json',dict(accounts=[dict(mechanism=m,account_commit=git('rev-parse',a),implementation_commits=[git('rev-parse',i) for i in ii]) for m,a,ii in accounts],files=modified,full_tested_production=binding['revision'],ownership='Sequence model/parser/renderer only. No parse/mod, ASCII, class, TIM, shared text/style/font or target edits. Accounts retain original dates; follow-up account is dated2026-11-05.'))
models=read(C/'final-model-and-ascii.json');write('model-and-ascii.json.gz',models);write('model-ascii-summary.json',dict(entries=len(models),changed_ascii=[r['source'] for r in models if not r['ascii_equal']],note='Actual CLI -ttxt outputs change on14 label-normalization/negative cases; renderer not edited. No ASCII parity/invariance claim. Raw models/status/stdout/stderr retained.'))
print(json.dumps(dict(before=summary['before'],after=summary['after'],main=summary['main_library_tiers'],units=summary['units'],standing=summary['standing_perturbations']),indent=2))
