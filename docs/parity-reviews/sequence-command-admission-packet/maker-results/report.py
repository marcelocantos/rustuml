# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,re,subprocess,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();BASE='dc438fd0cd0f5edd44816db81f0d765bf48ce4e6'
def read(p):
 p=Path(p);return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_text())
def write(n,v): (C/n).write_text(json.dumps(v,indent=2)+'\n')
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],text=True).strip()
b=read(C/'before3533/measurements.json')+read(C/'before-maker28/measurements.json')+read(C/'before-size8/measurements.json');a=read(C/'final3561/measurements.json')+read(C/'final-size8/measurements.json')
assert len(a)==len(b)==3569
for x,y in zip(b,a):
 assert (x['source'],x['source_sha256'],x['golden_sha256'])==(y['source'],y['source_sha256'],y['golden_sha256'])
 assert sha(R/y['source'])==y['source_sha256'] and sha(R/y['golden'])==y['golden_sha256']
write('before3569.json',b);write('after3569.json',a)
ranges=[('frozen3111',0,3111),('checker422',3111,3533),('maker28',3533,3561),('size8',3561,3569)]
def stats(rows):
 return dict(entries=len(rows),cli_status=dict(collections.Counter(x['cli_comparison']['exit'] for x in rows)),cli_exit=dict(collections.Counter(x['cli']['exit'] for x in rows)),product_library=dict(collections.Counter(x['product_library']['exit'] for x in rows)),assisted_library=dict(collections.Counter(x['assisted_library']['exit'] for x in rows)),cli_equals_library=sum(x['cli_equals_library'] for x in rows),panics=sum('panicked at' in x['cli']['stderr'] or x['cli']['exit']<0 for x in rows),timeouts=sum(x['cli']['exit']==124 for x in rows))
changes=[];gains=[];losses=[];newerrors=[];residual=[]
for x,y in zip(b,a):
 row=dict(source=y['source'],source_sha256=y['source_sha256'],before_status=x['cli_comparison']['exit'],after_status=y['cli_comparison']['exit'],before_exit=x['cli']['exit'],after_exit=y['cli']['exit'],before_svg_sha256=x['cli']['svg_sha256'],after_svg_sha256=y['cli']['svg_sha256'],after_diagnostic=y['cli']['stderr'])
 if row['before_status']!=row['after_status'] or row['before_svg_sha256']!=row['after_svg_sha256'] or row['before_exit']!=row['after_exit']:changes.append(row)
 if row['before_status']!=0 and row['after_status']==0:gains.append(row)
 if row['before_status']==0 and row['after_status']!=0:losses.append(row)
 if row['before_exit']==0 and row['after_exit']!=0 and row['after_status']!=2:newerrors.append(row)
 if row['after_status']!=0:residual.append(row)
approved=read(C/'parent-approved-exceptions.json');assert {(x['source'],x['source_sha256']) for x in newerrors}=={(x['source'],x['source_sha256']) for x in approved};assert len(newerrors)==15 and not losses
write('changes.json',changes);write('remaining-failures.json',residual)
# Classify every nonpositive entry using immutable reference classifications, not Rust output.
old=read(R/'docs/parity-reviews/sequence-command-contract-repair-packet/maker-results/rejection-contract.json')['results'];oldby={x['source']:x for x in old};oldhash={sha(R/x['source']):x for x in old if (R/x['source']).exists()}
fresh={x['source']:x for x in read(R/'docs/parity-reviews/sequence-command-contract-repair-packet/checker-2b056ca1-astra/fresh-after-all.json.gz')}
maker={Path(x['name']).stem:x for x in read(C/'reference-classification.json')}
contracts=[]
for i,(x,y) in enumerate(zip(b,a)):
 if y['cli_comparison']['exit']!=2:continue
 if i<3111:
  info=oldby.get(y['source'],oldhash.get(y['source_sha256']));assert info,y['source'];kind=info['reference_kind'];intended=info['intended_valid'];je=info['java_exit'];ref=info
 elif i<3533:
  info=fresh[y['source']];kind='java_export_error' if info['reference_outcome']=='java_export_error' else 'java_invalid';intended=info['intended_valid'];je=info['java_exit'];ref={k:v for k,v in info.items() if k not in ['before','after']}
 else:
  info=maker[Path(y['source']).stem];assert not info['observed_valid'];kind='java_invalid';intended=info['intended_valid'];je=info['java_exit'];ref=info
 contracts.append(dict(source=y['source'],source_sha256=y['source_sha256'],cohort=next(n for n,l,h in ranges if l<=i<h),reference_kind=kind,intended_valid=intended,java_exit=je,reference_metadata=ref,before=x['cli'],after=y['cli'],rejection_before=x['cli']['exit']!=0,rejection_after=y['cli']['exit']!=0,lost_rejection=x['cli']['exit']!=0 and y['cli']['exit']==0))
assert len(contracts)==241 and not any(x['lost_rejection'] for x in contracts)
kinds={}
for k in sorted({x['reference_kind'] for x in contracts}):
 rows=[x for x in contracts if x['reference_kind']==k];kinds[k]=dict(entries=len(rows),before_rejected=sum(x['rejection_before'] for x in rows),after_rejected=sum(x['rejection_after'] for x in rows),false_successes=[x['source'] for x in rows if not x['rejection_after']])
write('rejection-contract.json',dict(entries=len(contracts),kinds=kinds,lost_rejections=0,results=contracts,note='Exporter failures are nonpositive, not a requirement to crash Rust. Historical nonpositive references preserve unknown original exit/validity rather than inventing it. Rejection improvements never count as positive SVG matches.'))
# Full stdout AST/ASCII/YAML output changes, including formerly failing cases.
mb=read(C/'before-all3533-models.json')+read(C/'before-maker28-models.json');ma=read(C/'final-all3561-models.json');sd=read(C/'size-diagnostics.json')
for rev,rows in [('before',mb),('restored',ma)]:
 rows.extend(dict(source=x['source'],source_sha256=x['source_sha256'],results={m:x['results'][m] for m in ['model','ascii','yaml']}) for x in sd if x['revision']==rev)
write('before3569-models.json',mb);write('after3569-models.json',ma)
gb=read(C/'before-golden-cli/results.json');ga=read(C/'final-golden-cli/results.json');gcb=read(C/'before-golden-cli/comparisons.json');gca=read(C/'final-golden-cli/comparisons.json');gmb=read(C/'before-golden12550-models.json');gma=read(C/'final-golden12550-models.json')
model_summary={};model_changes=[];family_changes=[]
for cohort,bm,am in [('retained3569',mb,ma),('golden12550',gmb,gma)]:
 assert len(bm)==len(am)
 for mode in ['model','ascii','yaml']:
  delta=[]
  for x,y in zip(bm,am):
   assert (x['source'],x['source_sha256'])==(y['source'],y['source_sha256']);u=x['results'][mode];v=y['results'][mode]
   if u['stdout']!=v['stdout'] or u['exit']!=v['exit'] or u['stderr']!=v['stderr']:
    r=dict(cohort=cohort,source=y['source'],mode=mode,before_exit=u['exit'],after_exit=v['exit'],stdout_changed=u['stdout']!=v['stdout'],stderr_changed=u['stderr']!=v['stderr'],before_sha256=u['stdout_sha256'],after_sha256=v['stdout_sha256']);delta.append(r);model_changes.append(r)
   if mode=='model':
    uf=u['stdout'].split('(',1)[0].strip() if u['exit']==0 else 'ERROR';vf=v['stdout'].split('(',1)[0].strip() if v['exit']==0 else 'ERROR'
    if uf!=vf:family_changes.append(dict(cohort=cohort,source=y['source'],before_family=uf,after_family=vf,after_diagnostic=v['stderr']))
  model_summary[cohort+':'+mode]=dict(entries=len(bm),stdout_changes=sum(x['stdout_changed'] for x in delta),diagnostic_changes=sum(x['stderr_changed'] for x in delta),new_errors=sum(x['before_exit']==0 and x['after_exit']!=0 for x in delta),new_successes=sum(x['before_exit']!=0 and x['after_exit']==0 for x in delta))
write('model-changes.json',model_changes);write('model-summary.json',model_summary)
# Include comparator result for each changed diagram family.
status={r['source']:r['comparison']['exit'] for r in gca}|{r['source']:r['cli_comparison']['exit'] for r in a}
for x in family_changes:x['after_comparison']=status[x['source']]
write('family-changes.json',family_changes)
gchanges=[];gnewerrors=[];gloss=[]
for x,y,u,v in zip(gb,ga,gcb,gca):
 assert x['source']==y['source']==u['source']==v['source'];assert x['source_sha256']==y['source_sha256'] and x['golden_sha256']==y['golden_sha256']
 if x['exit']==0 and y['exit']!=0:
  row=dict(source=x['source'],source_sha256=x['source_sha256'],before_exit=x['exit'],after_exit=y['exit'],before_comparison=u['comparison']['exit'],after_comparison=v['comparison']['exit'],diagnostic=y['stderr']);gnewerrors.append(row)
 if u['comparison']['exit']==0 and v['comparison']['exit']!=0:gloss.append(x['source'])
 if x['stdout_sha256']!=y['stdout_sha256']:gchanges.append(dict(source=x['source'],before_sha256=x['stdout_sha256'],after_sha256=y['stdout_sha256'],before_exit=x['exit'],after_exit=y['exit'],before_comparison=u['comparison']['exit'],after_comparison=v['comparison']['exit']))
assert not gloss and all(x['after_comparison']==2 for x in gnewerrors)
write('golden-byte-changes.json',gchanges);write('golden-new-rejections.json',gnewerrors)
# Inventory actual warnings by Java command class, retaining each diagnostic and case.
residue=[]
for cohort,rows in [('retained3569',a),('golden12550',ga)]:
 for x in rows:
  err=x['cli']['stderr'] if 'cli' in x else x['stderr']
  for line in err.split('\n'):
   match=re.match(r'warning: sequence command (\w+)(.*)',line)
   if match:residue.append(dict(cohort=cohort,source=x['source'],command_class=match[1],diagnostic=line))
write('unimplemented-command-residue.json',dict(by_class=dict(collections.Counter(x['command_class'] for x in residue)),entries=residue,note='Counts are CLI warning instances; input overlap retained. Recognized but unsupported commands are not counted as syntax errors or claimed implemented.'))
# Source identity permits retaining the already completed broad run, without rerunning it after exact reversal.
fb=read(C/'final-binding.json');changed=git('diff','--name-only',fb['revision'],'HEAD').splitlines();assert all(x.startswith('docs/parity-reviews/') for x in changed)
for p,h in fb['source_files'].items():assert sha(R/p)==h
for n,h in fb['binaries'].items():assert sha(C/'final-bin'/n)==h
write('final-source-identity.json',dict(final_production_revision=git('rev-parse','HEAD'),tested_binary_source_revision=fb['revision'],source_files=fb['source_files'],binaries=fb['binaries'],intervening_diff_paths=changed,production_source_byte_identical=True,broad_rerun=False,reason='ba3d guard reverted exactly by c525; all production/build/test sources are identical to tested268fa. Completed gates and raw outputs retained at their original revision, no relabelled execution. Supplemental eight cases ran through the exact source-equivalent binary.'))
log=(C/'final-logs/main-tiers-and-guards.log').read_text();section=log[log.index('held-out no-oracle perturbations diverged'):];failed=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',section,re.M));standing=len(list((R/'test-diagrams/perturbations').rglob('*.puml')))
write('standing-failures.json',dict(entries=standing,failures=sorted(failed),failure_count=len(failed),cwd=str(R/'crates/rustuml-oracle'),note='Built-in library perturbation suite is separate from repo-cwd real CLI; every nonzero retained.'))
summary=dict(kind='Maker measurement, NOT independent acceptance',baseline=BASE,production=git('rev-parse','HEAD'),tested_revision=fb['revision'],before=stats(b),after=stats(a),cohorts={n:dict(before=stats(b[l:h]),after=stats(a[l:h])) for n,l,h in ranges},gains=gains,lost_matches=losses,new_valid_errors=newerrors,parent_approved_exceptions=approved,lost_rejections=0,rejection_contract=kinds,golden=dict(entries=len(ga),sequence_sources=sum('/sequence/' in x['source'] for x in ga),before_comparison=dict(collections.Counter(x['comparison']['exit'] for x in gcb)),after_comparison=dict(collections.Counter(x['comparison']['exit'] for x in gca)),lost_matches=gloss,new_valid_errors=[],new_nonpositive_rejections=len(gnewerrors),svg_byte_changes=len(gchanges),model_summary={k:v for k,v in model_summary.items() if k.startswith('golden')},comparator_nonUTF8_panics=100,product_panics=sum('panicked at' in x['stderr'] or x['exit']<0 for x in ga),timeouts=sum(x['timeout'] for x in ga),note='100 comparator read panics are non-UTF8 reference files, not Rust CLI panics. 1199 comparator nonpositives plus100 non-UTF8 are1299 main exclusions, unchanged.'),main_library=dict(product=11181,assisted=10497,eligible=11251,sequence='1411/1411',label='Library API diagnostics, not11251 CLI invocations',failure_name_snapshot='final-failure-names'),units=dict(parser=589,renderer=801,cli=13,ascii_included=len(re.findall(r'^test ascii::', (C/'final-logs/units.log').read_text(),re.M))),guards='4/6',standing=dict(entries=standing,failures=len(failed)),stable_broad_runs=1,model_summary=model_summary)
write('summary.json',summary)
write('cohort-overlap.json',dict(entries=len(a),unique_paths=len({x['source'] for x in a}),unique_source_hashes=len({x['source_sha256'] for x in a}),duplicate_source_entries=len(a)-len({x['source_sha256'] for x in a}),cohorts={n:dict(entries=h-l,unique_source_hashes=len({x['source_sha256'] for x in a[l:h]}),overlap_prior_source_hashes=len({x['source_sha256'] for x in a[l:h]}&{x['source_sha256'] for x in a[:l]})) for n,l,h in ranges},overlap_golden_source_hashes=len({x['source_sha256'] for x in a}&{x['source_sha256'] for x in ga}),note='3111+422+28+8=3569; duplicates retained, not deduplicated or added to parent current corpus. Parent measures integration separately.'))
print(json.dumps({k:summary[k] for k in ['before','after','golden','units','standing','model_summary']},indent=2));print('rejections', {k:{n:v for n,v in x.items() if n!='false_successes'} for k,x in kinds.items()})
