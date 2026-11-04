# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Preserve measured maker evidence; no Java launches or checker attestation."""
import collections,gzip,hashlib,json,re,shutil,subprocess,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/sequence-filtered-display-packet/maker-results';D.mkdir(parents=True,exist_ok=True)
BASE='0cde5d7e55139b63cb8e91f8e42acd473441f80a';REVIEW='bd40c0f16838359779e406592edb51f32be2e9bd'
read=lambda p:json.loads(p.read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],text=True).strip()
def write(n,v):
 p=D/n;p.parent.mkdir(parents=True,exist_ok=True);b=(json.dumps(v,indent=2)+'\n').encode();p.write_bytes(gzip.compress(b,mtime=0) if n.endswith('.gz') else b)
def stats(rows):
 return dict(entries=len(rows),match=sum(r['cli_comparison']['exit']==0 for r in rows),svg_mismatch=sum(r['cli_comparison']['exit']==1 for r in rows),invalid_reference=sum(r['cli_comparison']['exit']==2 for r in rows),valid_non_svg_error=sum(r['cli_comparison']['exit'] not in [0,1,2] for r in rows),cli_exits=dict(collections.Counter(r['cli']['exit'] for r in rows)),invalid_cli_exits=dict(collections.Counter(r['cli']['exit'] for r in rows if r['cli_comparison']['exit']==2)),valid_errors=[r['source'] for r in rows if r['cli_comparison']['exit']!=2 and r['cli']['exit']],cli_panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']],library_render_panics=[r['source'] for r in rows if r['library_render_exit']==101],mode_exits={m:dict(collections.Counter(r[m]['exit'] for r in rows)) for m in ['cli_comparison','product_library','assisted_library']})
before=read(C/'before2345/measurements.json')+read(C/'before26/measurements.json');after=read(C/'final2371/measurements.json');assert len(before)==len(after)==2371
changes=[]
for a,b in zip(before,after):
 assert a['source']==b['source']
 for key in ['source','golden']:assert a[key+'_sha256']==b[key+'_sha256']==sha((R/b[key]).read_bytes())
 changes.append(dict(source=b['source'],cohort=b['cohort'],before_status=a['cli_comparison']['exit'],after_status=b['cli_comparison']['exit'],before_cli=a['cli']['exit'],after_cli=b['cli']['exit'],before_svg_sha256=a['cli']['svg_sha256'],after_svg_sha256=b['cli']['svg_sha256']))
losses={m:[b['source'] for a,b in zip(before,after) if a[m]['exit']==0 and b[m]['exit']!=0] for m in ['cli_comparison','product_library','assisted_library']};assert not any(losses.values())
new_errors=[b['source'] for a,b in zip(before,after) if b['cli_comparison']['exit']!=2 and a['cli']['exit']==0 and b['cli']['exit']];assert not new_errors
for n,v in [('before-measurements.json.gz',before),('measurements.json.gz',after),('before-after.json.gz',changes),('remaining-failures.json.gz',[x for x in after if x['cli_comparison']['exit']!=0]),('negative-status.json',[x for x in changes if x['after_status']==2])]:write(n,v)
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
for n in ['reference-results.json.gz','reference-transport.json','reference-manifest.json','reference-input-bundle.json.gz','reference-classification.json','parent-relay-input.json.gz','parent-upload-validation.json','parent-relay-validation.json','before-binding.json','candidate1-binding.json','final-binaries.json']:
 shutil.copy2(C/n,D/n)
for n in ['semantic-inventory','model-event-counts','creation-witnesses','final-model-and-ascii','yaml-changes']:
 write(n+'.json.gz',read(C/(n+'.json')))
write('semantic-summary.json',read(C/'semantic-summary.json'))
for p in C.glob('*.py'):
 if p.name not in ['accounts.py','edit.py','tests_and_notes.py']:shutil.copy2(p,D/p.name)
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
write('binaries.json',dict(before=read(C/'before-binding.json'),before_revision=BASE,after=binding,notes='Before binaries copied from the prior source-identical d714 production. Final clean production 61e6aeed6. Frozen compare_cli copied byte-identically. Candidate1 aa5ac30e6 was measured before the note terminator correction; outputs retained.'))
write('checks.json',read(C/'final-checks.json'))
log=(C/'final-logs/main-tiers-and-guards.log').read_text();prod=re.search(r'golden_no_oracle: (\d+)/(\d+)',log);ass=re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped',log);seq=re.search(r'^  sequence\s+(\d+)/(\d+)',log,re.M);assert prod and ass and seq
main_changes={}
for n in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 old=gzip.decompress((R/'docs/parity-reviews/sequence-external-display-packet/maker-results/logs/final-failure-names'/(n+'.gz')).read_bytes());new=(C/'final-failure-names'/n).read_bytes();main_changes[n]=dict(recovered=sorted(set(old.decode().splitlines())-set(new.decode().splitlines())),newly_failing=sorted(set(new.decode().splitlines())-set(old.decode().splitlines())),sha256=sha(new),snapshot='Copied immediately after final main/guard/standing command at 61e6aeed6; before is immutable 0cde maker snapshot.')
assert all(not v['newly_failing'] for v in main_changes.values())
units=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/units.log').read_text());cli=re.findall(r'test result: ok\. (\d+) passed;', (C/'final-logs/cli.log').read_text());ascii_count=len(re.findall(r'^test ascii::', (C/'final-logs/units.log').read_text(),re.M))
section=log[log.index('held-out no-oracle perturbations diverged'):];fails=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',section,re.M));registered=len(list((R/'test-diagrams/perturbations').rglob('*.puml')))
ranges=[('frozen2281',0,2281),('checker64',2281,2345),('maker24',2345,2369),('creole_handoff2',2369,2371)]
summary=dict(kind='Maker measurements only, not acceptance; NEW independent checker required',baseline=BASE,production=PRODUCTION,accounts=git('rev-parse','ca2aae93f'),note_terminator_account=git('rev-parse','5d4858fb2'),before=stats(before),after=stats(after),cohorts={n:dict(before=stats(before[a:b]),after=stats(after[a:b])) for n,a,b in ranges},gains=[x for x in changes if x['before_status'] not in [0,2] and x['after_status']==0],losses_by_mode=losses,new_valid_cli_errors=new_errors,main_library_tiers=dict(product_pass=int(prod[1]),eligible=int(prod[2]),assisted_pass=int(ass[2]),assisted_failures=int(ass[3]),skipped=int(ass[4]),sequence=f'{seq[1]}/{seq[2]}',changes=main_changes,label='Library APIs, not 11251 CLI subprocesses'),units=dict(parser=int(units[0]),renderer=int(units[1]),cli_pipe=int(cli[0]),ascii_included=ascii_count),guards='4/6; historic evidence/provenance red gates plus unreviewed new accounts remain red',standing_perturbations=dict(entries=registered,failures=len(fails),matches=registered-len(fails),cwd=str(R/'crates/rustuml-oracle')),broad_runs=1)
write('summary.json',summary)
overlap={}
for n,a,b in ranges:
 subset=after[a:b];overlap[n]=dict(entries=len(subset),unique_sources=len({x['source_sha256'] for x in subset}),overlap_prior_bytes=len({x['source_sha256'] for x in subset}&{x['source_sha256'] for x in after[:a]}))
write('cohort-overlap.json',dict(entries=len(after),unique_paths=len({x['source'] for x in after}),unique_source_bytes=len({x['source_sha256'] for x in after}),duplicate_entries=len(after)-len({x['source_sha256'] for x in after}),cohorts=overlap,parent1449=dict(overlap_paths=1123,parent_only_paths_not_replayed=326,overlap_unique_bytes=1097,parent_only_unique_bytes=278,provenance='Historical independently recorded overlap retained, not a new parent inventory or additive denominator.'),note='No duplicate removal; 2281+64+24+2=2371 entries. Parent cohort overlaps and must not be summed.'))
old=git('ls-tree','-r','--name-only',BASE,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();changed=set(git('diff','--name-only',BASE,'HEAD').splitlines());assert not set(old)&changed
imported=git('diff-tree','--no-commit-id','--name-only','-r',REVIEW).splitlines()
for p in imported:assert subprocess.check_output(['git','show',REVIEW+':'+p])==(R/p).read_bytes()
modified=git('diff','--diff-filter=M','--name-only','7dcfb7065','HEAD').splitlines();allowed=['crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-render/src/ascii.rs'];assert set(modified)<=set(allowed)
ascii_diff=git('diff',BASE,'HEAD','--','crates/rustuml-render/src/ascii.rs');added=[x for x in ascii_diff.splitlines() if x.startswith('+') and not x.startswith('+++')];assert len(added)==3 and sum('hide_stereotypes: false' in x for x in added)==2 and sum('stereotype_position_bottom: false' in x for x in added)==1
(D/'ascii-authorized.diff').write_text(ascii_diff+'\n');write('protected-evidence.json',dict(import_commit=REVIEW,local_import=git('rev-parse','7dcfb7065'),prior_evidence_paths=len(old),imported_paths=len(imported),prior_and_imported_byte_unchanged=True,modified_existing_files=modified,ascii_authorization='User explicitly authorized hide_stereotypes:false in exactly two SequenceDiagram test fixtures and stereotype_position_bottom:false in exactly one Participant test fixture, no renderer edits/no behavior claim.',ascii_diff='ascii-authorized.diff',comparator_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),measure_sha256=sha((R/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes())))
write('implementation-map.json',dict(mechanisms=[dict(mechanism=m,account_commit=git('rev-parse',a),implementation_commit=git('rev-parse',i)) for m,a,i in [('sequence-filtered-ordered-participant-display','ca2aae93f','7c05cfcf8'),('sequence-note-aggregate-paint-height','ca2aae93f','fc71fafc'),('sequence-external-note-owner','ca2aae93f','aa5ac30e6'),('sequence-note-shape-terminator','5d4858fb2','61e6aeed6')]],files=modified,ascii_authorization='Exactly three mechanical test constructor defaults in 7c05cfcf8; exact diff in ascii-authorized.diff. Actual CLI ASCII outputs retained separately.'))
models=read(C/'final-model-and-ascii.json');ys={}
for n,a,b in ranges:
 sub=models[a:b];ys[n]=dict(entries=len(sub),stdout_equal=sum(x['results']['before']['yaml']['stdout']==x['results']['after']['yaml']['stdout'] for x in sub),before_exits=dict(collections.Counter(x['results']['before']['yaml']['exit'] for x in sub)),after_exits=dict(collections.Counter(x['results']['after']['yaml']['exit'] for x in sub)))
yaml_changed=[x['source'] for x in models if x['results']['before']['yaml']['stdout']!=x['results']['after']['yaml']['stdout']];yaml_panics=[x['source'] for x in models if x['results']['after']['yaml']['exit']==101]
write('model-ascii-yaml-summary.json',dict(entries=len(models),yaml_by_cohort=ys,yaml_stdout_changes=yaml_changed,yaml_existing_panics=yaml_panics,yaml_explanation='All 2281 frozen full YAML stdout captures byte-identical, including empty stdout on errors. Both new fields are serde-default and omitted when false; old models round-trip unchanged. Thirty fresh/supplement stdout changes:16 nondefault Display fields,10 external Note events,4 valid shape terminator parse recoveries. Nineteen pre-existing --yaml panics remain in both revisions: CLI main.rs serializing nested enums in YAML is unsupported; per-process PIDs change stderr and are preserved, not new render panics.',changed_ascii=[x['source'] for x in models if not x['ascii_equal']],ascii_note='Ten actual output/status changes from parser ownership/terminator repairs; no ASCII behavior-invariance or parity claim. Only three fixture defaults touched. Complete AST/ASCII/YAML streams and statuses retained.'))
v=read(C/'reference-results.json');write('local-reference-validation.json',dict(cases=24,observed_valid=24,reference_gzip_sha256=sha((C/'reference-results.json.gz').read_bytes()),transport_sha256=sha((C/'reference-transport.json').read_bytes()),environment=v['environment'],validation_script='import_references.py',limits='Java argv derived from wrapper, not process traced. OpenJDK21.0.5/Graphviz12.2 differ from historical origin; transport conformance is scoped, not global font/Graphviz equivalence. No local Java/javac or own SSH.'))
with tarfile.open(D/'captures.tar.gz','w:gz') as t:
 for p in sorted(C.rglob('*')):
  rel=p.relative_to(C)
  if p.is_file() and not rel.parts[0].endswith('-bin'):t.add(p,arcname=str(rel))
print(json.dumps(dict(before=summary['before'],after=summary['after'],main=summary['main_library_tiers'],units=summary['units'],standing=summary['standing_perturbations'],overlap=read(D/'cohort-overlap.json')),indent=2))
