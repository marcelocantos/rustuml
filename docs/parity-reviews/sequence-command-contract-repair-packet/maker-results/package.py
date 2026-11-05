# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package maker measurements without authoring acceptance records."""
import collections,gzip,hashlib,json,re,shutil,subprocess,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/sequence-command-contract-repair-packet/maker-results';D.mkdir(parents=True,exist_ok=True)
BASE='9784e412146e3ece634d88ea0a1bddd6b42a82b6';REVIEW='e654acb9ba31aec53329c739e8151d5bf46659f7'
read=lambda p:json.loads(p.read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],text=True).strip()
def write(n,v):
 p=D/n;p.parent.mkdir(parents=True,exist_ok=True);b=(json.dumps(v,indent=2)+'\n').encode();p.write_bytes(gzip.compress(b,mtime=0) if n.endswith('.gz') else b)
def stats(rows):
 return dict(entries=len(rows),match=sum(r['cli_comparison']['exit']==0 for r in rows),svg_mismatch=sum(r['cli_comparison']['exit']==1 for r in rows),nonpositive_reference=sum(r['cli_comparison']['exit']==2 for r in rows),valid_cli_error=sum(r['cli_comparison']['exit'] not in [0,1,2] for r in rows),valid_error_sources=[r['source'] for r in rows if r['cli_comparison']['exit']!=2 and r['cli']['exit']],cli_panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']],library_diagnostic_panics=[dict(source=r['source'],stderr=r['product_library']['stderr']) for r in rows if r['library_render_exit']==101],mode_exits={m:dict(collections.Counter(r[m]['exit'] for r in rows)) for m in ['cli_comparison','product_library','assisted_library']})
before=read(C/'before3111/measurements.json');after=read(C/'final3111/measurements.json');assert len(before)==len(after)==3111
changes=[]
for a,b in zip(before,after):
 assert a['source']==b['source']
 for key in ['source','golden']:assert a[key+'_sha256']==b[key+'_sha256']==sha((R/b[key]).read_bytes())
 changes.append(dict(source=b['source'],cohort=b['cohort'],before_status=a['cli_comparison']['exit'],after_status=b['cli_comparison']['exit'],before_cli=a['cli']['exit'],after_cli=b['cli']['exit'],before_svg_sha256=a['cli']['svg_sha256'],after_svg_sha256=b['cli']['svg_sha256']))
losses={m:[b['source'] for a,b in zip(before,after) if a[m]['exit']==0 and b[m]['exit']!=0] for m in ['cli_comparison','product_library','assisted_library']};assert not any(losses.values())
new_errors=[b['source'] for a,b in zip(before,after) if b['cli_comparison']['exit']!=2 and a['cli']['exit']==0 and b['cli']['exit']];assert not new_errors
contract=read(C/'rejection-contract.json');assert contract['entries']==165 and not contract['lost_rejections']
for n,v in [('before-measurements.json.gz',before),('measurements.json.gz',after),('before-after.json.gz',changes),('remaining-failures.json.gz',[r for r in after if r['cli_comparison']['exit']!=0]),('rejection-contract.json',contract)]:write(n,v)
manifest=[]
for row in after:
 item={k:row[k] for k in ['source','golden','cohort']}
 for k in ['source','golden']:
  p=Path(row[k])
  if p.is_absolute():
   q=D/'inputs'/p.parent.name/p.name;q.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,q);item[k]=str(q.relative_to(R))
 manifest.append(item)
write('all-inputs.json',manifest)
for dirname in ['reference-inputs','signature-inputs']:
 for p in (C/dirname).glob('*.stderr'):shutil.copy2(p,D/'inputs'/dirname/p.name)
for prefix in ['', 'signature-']:
 for n in ['reference-results.json.gz','reference-transport.json','reference-manifest.json','reference-input-bundle.json.gz','reference-classification.json','parent-relay-input.json.gz','parent-upload-validation.json','parent-relay-validation.json']:
  shutil.copy2(C/(prefix+n),D/(prefix+n))
for n in ['semantic-inventory','model-event-counts','final-model-and-ascii']:
 write(n+'.json.gz',read(C/(n+'.json')))
for n in ['semantic-summary.json','model-summary.json']:
 write(n,read(C/n))
for n in ['prepare.py','capture_models.py','import_references.py','import_signature_references.py','signature_controls.py','semantic_inventory.py','combine.py','diagnostics.py','final_checks.py','bind.py','report.py','intermediate_diagnostics.py','package.py']:
 p=C/n;raw=p.read_text();(D/n).write_text(raw if raw.startswith('# Copyright') else '# Copyright 2026 Marcelo Cantos\n# SPDX-License-Identifier: Apache-2.0\n'+raw)
for folder in ['candidate1-final-logs','candidate1-final-failure-names','candidate2-final-logs','candidate2-final-failure-names','final-logs','final-failure-names']:
 for p in (C/folder).iterdir():
  q=D/'logs'/folder/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
for p in C.glob('*.log'):
 q=D/'logs'/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
binding=read(C/'final-binding.json');PRODUCTION=binding['revision'];assert PRODUCTION==git('rev-parse','HEAD')
for p,h in binding['source_files'].items():assert sha((R/p).read_bytes())==h
for folder,b in [('final-bin',binding),('before-bin',read(C/'before-binding.json'))]:
 for n,h in b['binaries'].items():assert sha((C/folder/n).read_bytes())==h
assert binding['binaries']['compare_cli']==read(C/'before-binding.json')['binaries']['compare_cli']
for p,h in read(C/'before-binding.json')['source_files'].items():assert sha(subprocess.check_output(['git','show',BASE+':'+p]))==h
write('binaries.json',dict(before=read(C/'before-binding.json'),before_evidence_revision=BASE,after=binding,intermediate=[read(C/'candidate1-final-binding.json'),read(C/'candidate2-final-binding.json')],note='Exact private binary/source hashes; before production d491d4bda is source-identical to9784 evidence. Comparator copied byte-identically. No comparator/harness edit; jobs4.'))
write('checks.json',read(C/'final-checks.json'))
log=(C/'final-logs/main-tiers-and-guards.log').read_text();prod=re.search(r'golden_no_oracle: (\d+)/(\d+)',log);ass=re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped',log);seq=re.search(r'^  sequence\s+(\d+)/(\d+)',log,re.M);assert prod and ass and seq
main_changes={};intermediates=[]
for n in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 old=gzip.decompress((R/'docs/parity-reviews/sequence-consumer-contract-packet/maker-results/logs/final-failure-names'/(n+'.gz')).read_bytes());new=(C/'final-failure-names'/n).read_bytes();main_changes[n]=dict(recovered=sorted(set(old.decode().splitlines())-set(new.decode().splitlines())),newly_failing=sorted(set(new.decode().splitlines())-set(old.decode().splitlines())),sha256=sha(new),byte_equal=old==new,snapshot='Copied immediately after each exact-revision main/guard/standing command; before immutable9784 maker snapshot.')
 for folder in ['candidate1','candidate2']:
  raw=(C/(folder+'-final-failure-names')/n).read_bytes();intermediates.append(dict(candidate=folder,revision=read(C/(folder+'-final-binding.json'))['revision'],tier=n,newly_failing=sorted(set(raw.decode().splitlines())-set(old.decode().splitlines())),sha256=sha(raw)))
assert all(not v['newly_failing'] for v in main_changes.values())
write('intermediate-runs.json',dict(runs=intermediates,broad_execution_count=3,reasons=['5dccddca first broad run exposed29 colored-note losses, both tiers; source-account2a0441642 and production70bc20cf1 correct composite whitespace expansion.','70bc20cf1 rerun restores prior main counts; subsequent source audit finds full-command signature context leak. Four supplementary references prove2 intermediate lost passes, preserved in candidate2-signature4.','b4849a500 final broad run after full-command context correction, accountd4fc86b24.'],note='Every intermediate failure retained. Final stable code was gated once; repeats were justified by actual production changes.'))
units=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/units.log').read_text());cli=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/cli.log').read_text());ascii_count=len(re.findall(r'^test ascii::', (C/'final-logs/units.log').read_text(),re.M))
section=log[log.index('held-out no-oracle perturbations diverged'):];fails=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',section,re.M));registered=len(list((R/'test-diagrams/perturbations').rglob('*.puml')))
ranges=[('frozen2549',0,2549),('checker534',2549,3083),('maker24',3083,3107),('signature4',3107,3111)]
summary=dict(kind='Maker measurements only; NEW independent checker required, no acceptance',baseline=BASE,production=PRODUCTION,before=stats(before),after=stats(after),cohorts={n:dict(before=stats(before[a:b]),after=stats(after[a:b])) for n,a,b in ranges},gains=[x for x in changes if x['before_status'] not in [0,2] and x['after_status']==0],losses_by_mode=losses,new_valid_cli_errors=new_errors,rejection_contract={k:v for k,v in contract.items() if k!='results'},main_library_tiers=dict(product_pass=int(prod[1]),eligible=int(prod[2]),assisted_pass=int(ass[2]),assisted_failures=int(ass[3]),skipped=int(ass[4]),sequence=f'{seq[1]}/{seq[2]}',changes=main_changes,label='Library API diagnostics, not11251 CLI subprocesses'),units=dict(parser=int(units[0]),renderer=int(units[1]),cli_pipe=int(cli[0]),ascii_included=ascii_count),guards='4/6; decimal provenance and rejected/unreviewed review records remain red',standing_perturbations=dict(entries=registered,failures=len(fails),matches=registered-len(fails),cwd=str(R/'crates/rustuml-oracle')),stable_broad_runs=1,total_broad_runs=3)
write('summary.json',summary)
write('cohort-overlap.json',dict(entries=len(after),unique_paths=len({x['source'] for x in after}),unique_source_bytes=len({x['source_sha256'] for x in after}),duplicate_entries=len(after)-len({x['source_sha256'] for x in after}),cohorts={n:dict(entries=b-a,unique_sources=len({x['source_sha256'] for x in after[a:b]}),overlap_prior_bytes=len({x['source_sha256'] for x in after[a:b]}&{x['source_sha256'] for x in after[:a]})) for n,a,b in ranges},parent1449='Historical separate parent cohort, not additive to these retained cohorts; no new parent inventory or integration measurement claimed. See prior9784 overlap record.',note='2549+534+24+4=3111. Duplicates, syntax-invalid and exporter-failure references retained.'))
old=git('ls-tree','-r','--name-only',BASE,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();changed=set(git('diff','--name-only',BASE,'HEAD').splitlines());assert not set(old)&changed
imported=git('diff-tree','--no-commit-id','--name-only','-r',REVIEW).splitlines()
for p in imported:assert subprocess.check_output(['git','show',REVIEW+':'+p])==(R/p).read_bytes()
modified=git('diff','--diff-filter=M','--name-only','1cb5fa66d','HEAD').splitlines();allowed=['crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs'];assert set(modified)==set(allowed)
write('protected-evidence.json',dict(import_commit=REVIEW,local_import=git('rev-parse','1cb5fa66d'),prior_evidence_paths=len(old),imported_paths=len(imported),prior_and_imported_byte_unchanged=True,modified_existing_files=modified,model_and_ascii_files_unchanged=True,comparator_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),measure_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes())))
mapping=[('sequence-required-note-identity','4d84995c7','8cb61e5dc'),('sequence-command-fox-space','4d84995c7','fc3019e36'),('sequence-command-entry-grammar','4d84995c7','f48bafbc9'),('sequence-command-provenance-correction','4d84995c7','5dccddca'),('sequence-pattern2-class-composition','2a0441642','70bc20cf1'),('sequence-create-command-signature-scope','d4fc86b24','b4849a500')]
write('implementation-map.json',dict(mechanisms=[dict(mechanism=m,account_commit=git('rev-parse',a),implementation_commit=git('rev-parse',i)) for m,a,i in mapping],files=modified,ownership='Only owned parse/sequence.rs production and one renderer comment changed. No model fields or ASCII fixture/renderer edits; no factory/global consumption/shared files.',provenance_corrections=['CommandParticipantA and A2/A3/A4 own declaration grammar, not nonexistent CommandParticipant#getRegexConcat.','DrawableSetInitializer method is prepareParticipant, not createParticipant.','Literal-space entry gates were Rust-side; Java uses RegexLeaf whitespace.','CommandReferenceMultilinesOverSeveral uses Pattern2 directly: bypass FoxSignature, unlike single-line CommandReferenceOverSeveral.']))
write('local-reference-validation.json',dict(batches=[dict(prefix=p,cases=len(read(C/(p+'reference-classification.json'))),classification=read(C/(p+'reference-classification.json')),environment=read(C/(p+'reference-results.json'))['environment'],gzip_sha256=sha((C/(p+'reference-results.json.gz')).read_bytes()),transport_sha256=sha((C/(p+'reference-transport.json')).read_bytes())) for p in ['', 'signature-']],limits='Java argv derived, not process-traced. OpenJDK21.0.5/Graphviz12.2 differ from origin; no global equivalence claim. No local Java/javac or own SSH.'))
with tarfile.open(D/'captures.tar.gz','w:gz') as t:
 for p in sorted(C.rglob('*')):
  rel=p.relative_to(C)
  if p.is_file() and not any(part.endswith('-bin') for part in rel.parts):t.add(p,arcname=str(rel))
print(json.dumps(dict(before=summary['before'],after=summary['after'],main=summary['main_library_tiers'],units=summary['units'],standing=summary['standing_perturbations']),indent=2))
