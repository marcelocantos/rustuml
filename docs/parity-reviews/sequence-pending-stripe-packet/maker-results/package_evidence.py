# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package measured maker results; no reference execution or review attestation."""
import ast,base64,collections,gzip,hashlib,json,re,shutil,subprocess,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/sequence-pending-stripe-packet/maker-results'
BASE='711dbca7c5e29592395f0927881aabfc2517b217';IMPORT='2766d2c5425786df05cf62fed538003a6d2a52d3';PRODUCTION='653b1ed7a49784e6e6205f3aebf69a16278f8d78';ACCOUNT='4aaab169442b1e8a1d9e896d0cd3f019de6803d4'
MODES=['cli_comparison','product_library','assisted_library']
def read(p):return json.loads(p.read_text())
def sha(b):return hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],text=True).strip()
def write(n,v):
 p=D/n;p.parent.mkdir(parents=True,exist_ok=True);b=(json.dumps(v,indent=2)+'\n').encode();p.write_bytes(gzip.compress(b,mtime=0) if n.endswith('.gz') else b)
def stats(rows):
 return dict(entries=len(rows),matches=sum(r['cli_comparison']['exit']==0 for r in rows),valid_failures=sum(r['cli_comparison']['exit']==1 for r in rows),invalid=sum(r['cli_comparison']['exit']==2 for r in rows),invalid_cli_exits=dict(collections.Counter(r['cli']['exit'] for r in rows if r['cli_comparison']['exit']==2)),valid_cli_errors=[r['source'] for r in rows if r['cli_comparison']['exit']!=2 and r['cli']['exit']!=0],cli_panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']],render_panics=[r['source'] for r in rows if r['library_render_exit']==101],mode_exits={m:dict(collections.Counter(r[m]['exit'] for r in rows)) for m in MODES},diagnostic_panics={m:[r['source'] for r in rows if r[m]['exit']==101] for m in MODES[1:]},cli_equals_library=sum(r['cli_equals_library'] for r in rows))
D.mkdir(parents=True,exist_ok=True)
before=sum([read(C/n/'measurements.json') for n in ['before2147','before20','before-handoff']],[]);after=read(C/'final2169/measurements.json');assert len(before)==len(after)==2169
changes=[]
for a,b in zip(before,after):
 assert a['source']==b['source']
 for k in ['source','golden']:assert a[k+'_sha256']==b[k+'_sha256']==sha((R/b[k]).read_bytes())
 changes.append(dict(source=b['source'],golden=b['golden'],source_sha256=b['source_sha256'],golden_sha256=b['golden_sha256'],before_comparator=a['cli_comparison']['exit'],after_comparator=b['cli_comparison']['exit'],before_cli=a['cli']['exit'],after_cli=b['cli']['exit'],before_svg_sha256=a['cli']['svg_sha256'],after_svg_sha256=b['cli']['svg_sha256']))
losses={m:[b['source'] for a,b in zip(before,after) if a[m]['exit']==0 and b[m]['exit']!=0] for m in MODES};assert not any(losses.values());assert not stats(after)['valid_cli_errors'];assert not stats(after)['cli_panics'];assert not stats(after)['render_panics']
for n,v in [('before-measurements.json.gz',before),('measurements.json.gz',after),('before-after.json.gz',changes),('remaining-failures.json.gz',[r for r in after if r['cli_comparison']['exit']!=0]),('negative-status.json',[r for r in changes if r['after_comparator']==2])]:write(n,v)
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
# Independent local verification of immutable relay envelope and exact requests.
v=read(C/'reference-results.json');m=read(C/'reference-manifest.json');e=v['environment'];assert e['headless'];assert e['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053';assert e['manifest_sha256']==sha((C/'reference-manifest.json').read_bytes());assert e['env_overrides']=={}
z=(C/'reference-results.json.gz').read_bytes();assert sha(z)=='fe3e044a7f09e30bb04b2cd19c4879e924cc0d9d82e205f2a5ebe59fad465f54';assert gzip.decompress(z)==(C/'reference-results.json').read_bytes();assert sha((C/'reference-transport.json').read_bytes())=='4f3025eb604cb5d9fe079b9575911bb76cc85011c46f89dbc87085dcd799e8d4';assert base64.b64decode(read(C/'reference-transport.json')['data'])==z
for req,row in zip(m['cases'],v['results']):
 assert req['name']==row['name'];assert sha(Path(req['source']).read_bytes())==req['source_sha256']==row['source_sha256']==row['stdin_sha256'];assert row['cwd']==e['worker']+'/new';assert row['actual_wrapper_argv']==[row['cwd'] if x=='PRIVATE_CWD' else x for x in req['invocation']];assert row['timezone']=='GMT+11:00' and not row['timed_out'];assert row['actual_java_argv']==[e['java'],'-Djava.awt.headless=true','-Dfile.encoding=UTF-8','-Duser.language=en','-Duser.country=AU','-Duser.timezone=GMT+11:00','-jar',e['jar'],'-pipe','-tsvg']
 for key in ['stdout','stderr']:assert sha(base64.b64decode(row[key+'_base64'],validate=True))==row[key+'_sha256']
 for key,actual in [('argv','actual_wrapper_argv'),('java_argv','actual_java_argv')]:
  if key in row:assert (ast.literal_eval(row[key]) if isinstance(row[key],str) else row[key])==row[actual]
write('local-reference-validation.json',dict(cases=20,valid=16,command_errors=4,timeouts=0,manifest_sha256=e['manifest_sha256'],gzip_sha256=sha(z),transport_sha256=sha((C/'reference-transport.json').read_bytes()),environment=e,raw_preserved=True,limits='Recorded Java argv derives from pinned wrapper, not process tracing. OpenJDK21.0.5/Graphviz12.2 differ from origin; scoped transport conformance is not global font/Graphviz equivalence. Four XML error diagrams contain the incompatible pending CREATE diagnostic, not the literal Syntax Error string.'))
for n in ['reference-results.json.gz','reference-transport.json','reference-manifest.json','reference-input-bundle.json.gz','reference-classification.json','parent-relay-input.json.gz','parent-upload-validation.json','parent-relay-validation.json','before-binding.json','first-binding.json','final-binaries.json']:
 shutil.copy2(C/n,D/n)
for p in C.glob('*.py'):shutil.copy2(p,D/p.name)
for folder in ['final-logs','final-failure-names']:
 for p in (C/folder).iterdir():
  q=D/'logs'/folder/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
for p in C.glob('*.log'):
 q=D/'logs'/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
for name in ['semantic-inventory','model-event-counts','creation-witnesses','final-model-and-ascii']:
 write(name+'.json.gz',read(C/(name+'.json')))
write('semantic-summary.json',read(C/'semantic-summary.json'))
binding=read(C/'final-binaries.json');assert binding['revision']==PRODUCTION==git('rev-parse','HEAD')
for p,h in binding['source_files'].items():assert sha((R/p).read_bytes())==h
binding['binaries']['compare_cli']=sha((C/'final-bin/compare_cli').read_bytes());assert binding['binaries']['compare_cli']==read(C/'before-binding.json')['binaries']['compare_cli']
for folder,record in [('final-bin',binding),('before-bin',read(C/'before-binding.json'))]:
 for n,h in record['binaries'].items():assert sha((C/folder/n).read_bytes())==h
for p,h in read(C/'before-binding.json')['source_files'].items():assert sha(subprocess.check_output(['git','show',BASE+':'+p]))==h
write('binaries.json',dict(before=read(C/'before-binding.json'),before_revision=BASE,after=binding,binding='Before bins built at a4c3c357, source-identical production to711. Final bins built from clean653b1ed7. Frozen compare_cli binary copied unchanged after build; explicit hash retained. First dirty build has exact source snapshot and source/binary hashes; no unbound draft represented as clean revision.'))
write('checks.json',read(C/'final-checks.json'))
log=(C/'final-logs/main-tiers-and-guards.log').read_text();prod=re.search(r'golden_no_oracle: (\d+)/(\d+)',log);ass=re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped',log);seq=re.search(r'^  sequence\s+(\d+)/(\d+)',log,re.M);assert prod and ass and seq
main_changes={}
for n in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 old=gzip.decompress((R/'docs/parity-reviews/sequence-component-display-packet/maker-results/logs'/(n+'.gz')).read_bytes());new=(C/'final-failure-names'/n).read_bytes();main_changes[n]=dict(recovered=sorted(set(old.decode().splitlines())-set(new.decode().splitlines())),newly_failing=sorted(set(new.decode().splitlines())-set(old.decode().splitlines())),sha256=sha(new),snapshot='Copied immediately after main tiers and guard command; never read mutable repository names for packaging. Before is711 maker retained exact final snapshot, not checker overwritten intermediate names.')
assert all(not x['newly_failing'] for x in main_changes.values())
units=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/units.log').read_text());cli=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/cli.log').read_text());ascii_count=len(re.findall(r'^test ascii::', (C/'final-logs/units.log').read_text(),re.M))
section=log[log.index('held-out no-oracle perturbations diverged'):];fails=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',section,re.M));registered=len(list((R/'test-diagrams/perturbations').rglob('*.puml')))
cohort_ranges=[('preserved2083',0,2083),('checker64',2083,2147),('maker20',2147,2167),('handoff2',2167,2169)]
summary=dict(kind='Maker evidence only; new independent checker required',baseline=BASE,implementation=PRODUCTION,before=stats(before),after=stats(after),cohorts={n:dict(before=stats(before[a:b]),after=stats(after[a:b])) for n,a,b in cohort_ranges},gains=[r for r in changes if r['before_comparator']==1 and r['after_comparator']==0],losses_by_mode=losses,main_library_tiers=dict(product_pass=int(prod[1]),eligible=int(prod[2]),assisted_pass=int(ass[2]),assisted_failures=int(ass[3]),skipped=int(ass[4]),sequence=f'{seq[1]}/{seq[2]}',changes=main_changes,label='Library APIs, not11251 CLI subprocesses'),units=dict(parser=int(units[0]),renderer=int(units[1]),cli_pipe=int(cli[0]),ascii_included=ascii_count),guards='4/6',standing_perturbations=dict(entries=registered,failures=len(fails),matches=registered-len(fails),cwd=str(R/'crates/rustuml-oracle')),broad_runs=1)
write('summary.json',summary)
overlap={}
for n,a,b in cohort_ranges:
 rows=after[a:b];overlap[n]=dict(entries=len(rows),unique_sources=len({r['source_sha256'] for r in rows}),overlap_prior_bytes=len({r['source_sha256'] for r in rows}&{r['source_sha256'] for r in after[:a]}))
write('cohort-overlap.json',dict(entries=len(after),unique_paths=len({r['source'] for r in after}),unique_source_bytes=len({r['source_sha256'] for r in after}),duplicate_entries=len(after)-len({r['source_sha256'] for r in after}),cohorts=overlap,parent1449=dict(overlap_paths=1123,parent_only_paths_not_replayed=326,overlap_unique_bytes=1097,parent_only_unique_bytes=278,provenance='Immutable checker711 parent-overlap.json; historical parent overlap with frozen2083 only. No new parent inventory or additive claim.'),note='Retain all entries including duplicate bytes, invalids and separate shared handoffs.'))
old=git('ls-tree','-r','--name-only',BASE,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();changed=set(git('diff','--name-only',BASE,'HEAD').splitlines());assert not set(old)&changed
imp=git('diff-tree','--no-commit-id','--name-only','-r','96e2561122e70ffe53dfa3f102cf18d199cc464f').splitlines()
for p in imp:assert subprocess.check_output(['git','show','96e2561122e70ffe53dfa3f102cf18d199cc464f:'+p])==(R/p).read_bytes()
modified=git('diff','--diff-filter=M','--name-only',IMPORT,'HEAD').splitlines();allowed=['crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs'];assert set(modified)<=set(allowed)
write('protected-evidence.json',dict(import_commit='96e2561122e70ffe53dfa3f102cf18d199cc464f',local_import=IMPORT,prior_evidence_paths=len(old),imported_paths=len(imp),prior_and_imported_byte_unchanged=True,modified_existing_files=modified,ascii='No ASCII file changes or expanded ownership. Actual output changes from parser rejection retained.',comparator_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),measure_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes())))
accounts=['sequence-single-pending-create','sequence-queue-stripe-origin','sequence-message-stripe-baselines','sequence-self-half-polygon-order','sequence-collections-source-clarification']
write('implementation-map.json',dict(accounts=[dict(mechanism=m,account_commit=ACCOUNT,implementation_commits=['d5478061ffbd2b1ec31a8ac513ff23b4489e3fa6' if i==0 else PRODUCTION]) for i,m in enumerate(accounts)],files=modified,full_tested_production=PRODUCTION,ownership='Only sequence model/parser/renderer. No parse/mod, ASCII, class, TIM, shared text/style/fonts or targets.'))
models=read(C/'final-model-and-ascii.json');write('model-ascii-summary.json',dict(entries=len(models),changed_ascii=[r['source'] for r in models if not r['ascii_equal']],note='Four intended-invalid pending CREATE inputs now reject in both SVG and ASCII paths. All144 other measured ASCII outputs unchanged; no ASCII parity or global invariance claim. Raw model/status/output retained.'))
# Archive every private capture, including initial compile failure and draft output.
with tarfile.open(D/'captures.tar.gz','w:gz') as t:
 for p in sorted(C.rglob('*')):
  rel=p.relative_to(C)
  if p.is_file() and not rel.parts[0].endswith('-bin'):t.add(p,arcname=str(rel))
print(json.dumps(dict(before=summary['before'],after=summary['after'],main=summary['main_library_tiers'],units=summary['units'],standing=summary['standing_perturbations'],overlap=read(D/'cohort-overlap.json')),indent=2))
