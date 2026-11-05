# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package measured consumer contracts, never an acceptance attestation."""
import collections,gzip,hashlib,json,re,shutil,subprocess,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/sequence-consumer-contract-packet/maker-results';D.mkdir(parents=True,exist_ok=True)
BASE='bce6bea405d8b5efd55136f4768117115e7f634f';REVIEW='f3913f3cb7094d8aedd5618f1ba27830a2804713'
read=lambda p:json.loads(p.read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],text=True).strip()
def write(n,v):
 p=D/n;p.parent.mkdir(parents=True,exist_ok=True);b=(json.dumps(v,indent=2)+'\n').encode();p.write_bytes(gzip.compress(b,mtime=0) if n.endswith('.gz') else b)
def stats(rows):
 return dict(entries=len(rows),match=sum(r['cli_comparison']['exit']==0 for r in rows),svg_mismatch=sum(r['cli_comparison']['exit']==1 for r in rows),nonpositive_reference=sum(r['cli_comparison']['exit']==2 for r in rows),valid_non_svg_error=sum(r['cli_comparison']['exit'] not in [0,1,2] for r in rows),nonpositive_cli_exits=dict(collections.Counter(r['cli']['exit'] for r in rows if r['cli_comparison']['exit']==2)),valid_errors=[r['source'] for r in rows if r['cli_comparison']['exit']!=2 and r['cli']['exit']],cli_panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']],library_diagnostic_panics=[dict(source=r['source'],stderr=r['product_library']['stderr']) for r in rows if r['library_render_exit']==101],mode_exits={m:dict(collections.Counter(r[m]['exit'] for r in rows)) for m in ['cli_comparison','product_library','assisted_library']})
before=read(C/'before2549/measurements.json');after=read(C/'final2549/measurements.json');assert len(before)==len(after)==2549
changes=[]
for a,b in zip(before,after):
 assert a['source']==b['source']
 for key in ['source','golden']:assert a[key+'_sha256']==b[key+'_sha256']==sha((R/b[key]).read_bytes())
 changes.append(dict(source=b['source'],cohort=b['cohort'],before_status=a['cli_comparison']['exit'],after_status=b['cli_comparison']['exit'],before_cli=a['cli']['exit'],after_cli=b['cli']['exit'],before_svg_sha256=a['cli']['svg_sha256'],after_svg_sha256=b['cli']['svg_sha256']))
losses={m:[b['source'] for a,b in zip(before,after) if a[m]['exit']==0 and b[m]['exit']!=0] for m in ['cli_comparison','product_library','assisted_library']};assert not any(losses.values())
new_errors=[b['source'] for a,b in zip(before,after) if b['cli_comparison']['exit']!=2 and a['cli']['exit']==0 and b['cli']['exit']];assert not new_errors
contract=read(C/'rejection-contract.json');assert contract['entries']==99 and not contract['introduced_losses']
for x in contract['results']:
 i=next(i for i,r in enumerate(after) if r['source']==x['source'])
 for key,rows in [('before',before),('after',after)]:assert x['results'][key]['exit']==rows[i]['cli']['exit'] and x['results'][key]['stdout_sha256']==rows[i]['cli']['svg_sha256']
for n,v in [('before-measurements.json.gz',before),('measurements.json.gz',after),('before-after.json.gz',changes),('remaining-failures.json.gz',[r for r in after if r['cli_comparison']['exit']!=0]),('nonpositive-status.json',[r for r in changes if r['after_status']==2]),('rejection-contract.json',contract)]:write(n,v)
manifest=[]
for row in after:
 item={k:row[k] for k in ['source','golden','cohort']}
 for k in ['source','golden']:
  p=Path(row[k])
  if p.is_absolute():
   q=D/'inputs'/p.parent.name/p.name;q.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,q);item[k]=str(q.relative_to(R))
 manifest.append(item)
write('all-inputs.json',manifest)
for p in (C/'reference-inputs').glob('*.stderr'):shutil.copy2(p,D/'inputs/reference-inputs'/p.name)
for n in ['reference-results.json.gz','reference-transport.json','reference-manifest.json','reference-input-bundle.json.gz','reference-classification.json','parent-relay-input.json.gz','parent-upload-validation.json','parent-relay-validation.json','before-binding.json','candidate1-binding.json','final-binaries.json','measurement-label-recovery.json','nonpositive-contract-inputs.json']:
 shutil.copy2(C/n,D/n)
for n in ['semantic-inventory','model-event-counts','creation-witnesses','final-model-and-ascii']:
 write(n+'.json.gz',read(C/(n+'.json')))
write('semantic-summary.json',read(C/'semantic-summary.json'))
for n in ['prepare.py','capture_models.py','import_references.py','semantic_inventory.py','combine.py','final_checks.py','package.py']:
 shutil.copy2(C/n,D/n)
shutil.copy2(R/'docs/parity-reviews/sequence-filtered-display-packet/checker-bce6bea-astra-f535/support/rejection_contract.py',D/'rejection_contract.py')
for folder in ['final-logs','final-failure-names']:
 for p in (C/folder).iterdir():
  q=D/'logs'/folder/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
for p in C.glob('*.log'):
 q=D/'logs'/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
binding=read(C/'final-binaries.json');PRODUCTION=binding['revision'];assert PRODUCTION==git('rev-parse','HEAD')
for p,h in binding['source_files'].items():assert sha((R/p).read_bytes())==h
for folder,b in [('final-bin',binding),('before-bin',read(C/'before-binding.json'))]:
 for n,h in b['binaries'].items():assert sha((C/folder/n).read_bytes())==h
assert binding['binaries']['compare_cli']==read(C/'before-binding.json')['binaries']['compare_cli']
for p,h in read(C/'before-binding.json')['source_files'].items():assert sha(subprocess.check_output(['git','show',BASE+':'+p]))==h
write('binaries.json',dict(before=read(C/'before-binding.json'),before_revision=BASE,after=binding,note='Before binaries are retained source-identical61e6aeed6; final exact clean production d491d4bda. Frozen compare_cli copied byte-identically; no comparator or harness edit. Private target jobs4.'))
write('checks.json',read(C/'final-checks.json'))
log=(C/'final-logs/main-tiers-and-guards.log').read_text();prod=re.search(r'golden_no_oracle: (\d+)/(\d+)',log);ass=re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped',log);seq=re.search(r'^  sequence\s+(\d+)/(\d+)',log,re.M);assert prod and ass and seq
main_changes={}
for n in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 old=gzip.decompress((R/'docs/parity-reviews/sequence-filtered-display-packet/maker-results/logs/final-failure-names'/(n+'.gz')).read_bytes());new=(C/'final-failure-names'/n).read_bytes();main_changes[n]=dict(recovered=sorted(set(old.decode().splitlines())-set(new.decode().splitlines())),newly_failing=sorted(set(new.decode().splitlines())-set(old.decode().splitlines())),sha256=sha(new),byte_equal=old==new,snapshot='Copied immediately after stable main/guard/standing command; before is immutable bce6 maker snapshot.')
assert all(not v['newly_failing'] for v in main_changes.values())
units=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/units.log').read_text());cli=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/cli.log').read_text());ascii_count=len(re.findall(r'^test ascii::', (C/'final-logs/units.log').read_text(),re.M))
section=log[log.index('held-out no-oracle perturbations diverged'):];fails=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',section,re.M));registered=len(list((R/'test-diagrams/perturbations').rglob('*.puml')))
ranges=[('frozen2371',0,2371),('checker148',2371,2519),('maker30',2519,2549)]
summary=dict(kind='Maker measurements only, no acceptance; NEW independent checker required',baseline=BASE,production=PRODUCTION,account_commit=git('rev-parse','0b512262a'),before=stats(before),after=stats(after),cohorts={n:dict(before=stats(before[a:b]),after=stats(after[a:b])) for n,a,b in ranges},gains=[x for x in changes if x['before_status'] not in [0,2] and x['after_status']==0],losses_by_mode=losses,new_valid_cli_errors=new_errors,rejection_contract={k:v for k,v in contract.items() if k!='results'},main_library_tiers=dict(product_pass=int(prod[1]),eligible=int(prod[2]),assisted_pass=int(ass[2]),assisted_failures=int(ass[3]),skipped=int(ass[4]),sequence=f'{seq[1]}/{seq[2]}',changes=main_changes,label='Library APIs, not11251 CLI subprocesses'),units=dict(parser=int(units[0]),renderer=int(units[1]),cli_pipe=int(cli[0]),ascii_included=ascii_count),guards='4/6; decimal provenance and rejected/unreviewed review records remain red',standing_perturbations=dict(entries=registered,failures=len(fails),matches=registered-len(fails),cwd=str(R/'crates/rustuml-oracle')),broad_runs=1)
write('summary.json',summary)
overlap={}
for n,a,b in ranges:
 subset=after[a:b];overlap[n]=dict(entries=len(subset),unique_sources=len({x['source_sha256'] for x in subset}),overlap_prior_bytes=len({x['source_sha256'] for x in subset}&{x['source_sha256'] for x in after[:a]}))
write('cohort-overlap.json',dict(entries=len(after),unique_paths=len({x['source'] for x in after}),unique_source_bytes=len({x['source_sha256'] for x in after}),duplicate_entries=len(after)-len({x['source_sha256'] for x in after}),cohorts=overlap,parent1449=dict(overlap_paths=1123,parent_only_paths_not_replayed=326,overlap_unique_bytes=1097,parent_only_unique_bytes=278,provenance='Historical independently recorded overlap, not a new parent inventory or additive denominator.'),note='2371+148+30=2549; duplicates and nonpositive sources retained.'))
old=git('ls-tree','-r','--name-only',BASE,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();changed=set(git('diff','--name-only',BASE,'HEAD').splitlines());assert not set(old)&changed
imported=git('diff-tree','--no-commit-id','--name-only','-r',REVIEW).splitlines()
for p in imported:assert subprocess.check_output(['git','show',REVIEW+':'+p])==(R/p).read_bytes()
modified=git('diff','--diff-filter=M','--name-only','b286fe630','HEAD').splitlines();allowed=['crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs'];assert set(modified)==set(allowed)
write('protected-evidence.json',dict(import_commit=REVIEW,local_import=git('rev-parse','b286fe630'),prior_evidence_paths=len(old),imported_paths=len(imported),prior_and_imported_byte_unchanged=True,modified_existing_files=modified,model_and_ascii_files_unchanged=True,comparator_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),measure_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes())))
write('implementation-map.json',dict(mechanisms=[dict(mechanism=m,account_commit=git('rev-parse','0b512262a'),implementation_commit=git('rev-parse',i)) for m,i in [('sequence-pattern2-whitespace','df9078e2e'),('sequence-all-participant-display-consumers','f4b3db075'),('sequence-teoz-self-note-component','47fb7bd3a'),('sequence-note-over-command-domain','d491d4bda')]],files=modified,ownership='Only owned parse/sequence.rs and render/sequence.rs changed. No model fields or ASCII fixture/renderer edit. No parse/mod, global unconsumed handling or shared text/style/font edit.'))
models=read(C/'final-model-and-ascii.json');ys={}
for n,a,b in ranges:
 sub=models[a:b];ys[n]=dict(entries=len(sub),stdout_equal=sum(x['results']['before']['yaml']['stdout']==x['results']['after']['yaml']['stdout'] for x in sub),before_exits=dict(collections.Counter(x['results']['before']['yaml']['exit'] for x in sub)),after_exits=dict(collections.Counter(x['results']['after']['yaml']['exit'] for x in sub)))
model_changes=[dict(source=x['source'],results=x['results']) for x in models if any(x['results']['before'][mode]['stdout']!=x['results']['after'][mode]['stdout'] or x['results']['before'][mode]['exit']!=x['results']['after'][mode]['exit'] for mode in ['model','yaml','ascii'])];write('model-output-changes.json.gz',model_changes)
write('model-ascii-yaml-summary.json',dict(entries=len(models),yaml_by_cohort=ys,yaml_existing_panics=[x['source'] for x in models if x['results']['after']['yaml']['exit']==101],changed_ascii=[x['source'] for x in models if not x['ascii_equal']],note='Frozen2371 full YAML stdout is byte-identical. Fourteen additional output/status changes:12 restored negative rejections plus2 invalid whitespace messages dropped while CLI still succeeds (T14.17). Teoz effective shapes are renderer-local; requested AST/YAML shapes retained. Nineteen existing CLI nested-enum YAML serialization panics remain; process IDs change stderr. Separate checker-proven pre-existing state minty YAML deserialization panic is not rerun or conflated. No ASCII parity claim.'))
# Per-kind outcomes and complete counts are independent of whole-pass totals.
semantics=read(C/'semantic-inventory.json');by_source={x['source']:x for x in semantics};kinds={}
for kind in ['participant','actor','boundary','control','entity','database','collections','queue']:
 subset=[(a,b) for a,b in zip(before,after) if any(key in Path(b['source']).stem for key in ['indf535_hidden_'+kind+'_','indf535_bottom_'+kind+'_',kind+'_ordered_'])]
 kinds[kind]=dict(entries=len(subset),before=stats([a for a,b in subset]),after=stats([b for a,b in subset]),witnesses=[by_source[b['source']] for a,b in subset])
write('participant-kind-results.json.gz',kinds)
v=read(C/'reference-results.json');write('local-reference-validation.json',dict(cases=30,observed_valid=22,observed_invalid=8,reference_gzip_sha256=sha((C/'reference-results.json.gz').read_bytes()),transport_sha256=sha((C/'reference-transport.json').read_bytes()),environment=v['environment'],validation_script='import_references.py',limits='Java argv is wrapper-derived, not process-traced. OpenJDK21.0.5/Graphviz12.2 differ from origin; no global equivalence claim. No local Java/javac or own SSH.'))
with tarfile.open(D/'captures.tar.gz','w:gz') as t:
 for p in sorted(C.rglob('*')):
  rel=p.relative_to(C)
  if p.is_file() and not rel.parts[0].endswith('-bin'):t.add(p,arcname=str(rel))
print(json.dumps(dict(before=summary['before'],after=summary['after'],main=summary['main_library_tiers'],units=summary['units'],standing=summary['standing_perturbations'],overlap=read(D/'cohort-overlap.json')),indent=2))
