# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,json,collections,re,hashlib,subprocess,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-78c595f4');D=R/'docs/parity-reviews/sequence-pending-stripe-packet/maker-results'
def read(p):return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
def sha(b):return hashlib.sha256(b).hexdigest()
def cohort(r):
 return {'checker64':'checker64','new-maker20':'maker20','shared-handoff2':'handoff2'}.get(r['cohort'],'preserved')
def report(rows):
 out=dict(entries=len(rows),distinct_source_bytes=len(set(r['source_sha256'] for r in rows)),duplicates=len(rows)-len(set(r['source_sha256'] for r in rows)),by_cohort={},losses={},recoveries={},new_valid_parse_errors=[],new_renderer_panics=[],ascii_changed=[],ast_changed=[])
 for label in sorted(set(cohort(r) for r in rows)):
  sub=[r for r in rows if cohort(r)==label];out['by_cohort'][label]=dict(entries=len(sub),**{rev:dict(collections.Counter(r['results'][rev]['cli_comparison']['exit'] for r in sub)) for rev in ['before','after']})
 for mode in ['cli_comparison','product_library','assisted_library']:
  out['losses'][mode]=[r['source'] for r in rows if r['results']['before'][mode]['exit']==0 and r['results']['after'][mode]['exit']!=0]
  out['recoveries'][mode]=[r['source'] for r in rows if r['results']['before'][mode]['exit']!=0 and r['results']['after'][mode]['exit']==0]
 for r in rows:
  b,a=(r['results'][x] for x in ['before','after'])
  if a['cli_comparison']['exit']!=2:
   if a['cli']['exit']==1 and b['cli']['exit']!=1:out['new_valid_parse_errors'].append(r['source'])
   if 'panicked at' in a['cli']['stderr'] and 'panicked at' not in b['cli']['stderr']:out['new_renderer_panics'].append(r['source'])
  for m in ['ascii','ast']:
   if (a[m]['exit'],a[m]['sha256'])!=(b[m]['exit'],b[m]['sha256']):out[m+'_changed'].append(r['source'])
 out['invalid_cli_status']={v:dict(collections.Counter(r['results'][v]['cli']['exit'] for r in rows if r['results'][v]['cli_comparison']['exit']==2)) for v in ['before','after']}
 out['helper_panics']={v:{mode:[dict(source=r['source'],exit=r['results'][v][mode]['exit'],stderr=r['results'][v][mode]['stderr']) for r in rows if 'panicked at' in r['results'][v][mode]['stderr']] for mode in ['product_library','assisted_library']} for v in ['before','after']}
 return out
rows=read(C/'replay2169/measurements.json.gz');fresh=read(C/'fresh86/measurements.json.gz');summary=dict(frozen=report(rows),fresh86=report(fresh))
# Independent exact maker capture, status and hash comparison by source bytes and preserved entry index.
makers={v:read(D/(('before-measurements' if v=='before' else 'measurements')+'.json.gz')) for v in ['before','after']};comparisons={}
for v,m in makers.items():
 assert len(m)==len(rows)
 differences=[]
 for n,(a,b) in enumerate(zip(m,rows)):
  assert a['source_sha256']==b['source_sha256'] and a['golden_sha256']==b['golden_sha256']
  for mode in ['cli_comparison','product_library','assisted_library']:
   if a[mode]['exit']!=b['results'][v][mode]['exit']:differences.append(dict(index=n,source=b['source'],mode=mode,maker=a[mode]['exit'],checker=b['results'][v][mode]['exit']))
  if a['cli']['svg_sha256']!=b['results'][v]['cli']['sha256']:differences.append(dict(index=n,source=b['source'],mode='cli_svg_sha256'))
 comparisons[v]=differences
summary['maker_reproduction_differences']=comparisons
models=read(D/'final-model-and-ascii.json.gz');lookup={r['source_sha256']:r for r in rows};selected=[lookup[x['source_sha256']] for x in models];summary['maker148']=dict(entries=len(selected),ascii_changes=[r['source'] for r in selected if r['results']['before']['ascii']['sha256']!=r['results']['after']['ascii']['sha256']],unchanged=sum(r['results']['before']['ascii']['sha256']==r['results']['after']['ascii']['sha256'] for r in selected))
parent=read(R/'docs/parity-reviews/sequence-rejection-repair/checker-support/replay1449.json.gz');pres=[r for r in rows if cohort(r)=='preserved'];paths={r['source'] for r in pres};over=[x for x in parent if x['original_source'] in paths];un=[x for x in parent if x['original_source'] not in paths];summary['parent1449']=dict(entries=len(parent),overlapping_paths=len(over),unreplayed_paths=len(un),overlapping_unique_bytes=len({x['source_sha256'] for x in over}),unreplayed_path_unique_bytes=len({x['source_sha256'] for x in un}),unreplayed_exclusive_unique_bytes=len({x['source_sha256'] for x in un}-{x['source_sha256'] for x in over}),not_additive=True)
# Explicit semantic inventory includes raw AST event counts and heads/labels in all captured modes.
semantic=[]
for folder,rr in [('replay2169',rows),('fresh86',fresh)]:
 for row in rr:
  rec=dict(cohort=folder,source=row['source'],java=row['reference_inventory'],versions={})
  for rev in ['before','after']:
   result=row['results'][rev];ast=(C/folder/result['ast']['output']).read_text();events=ast.split('        events: [',1)[-1].split('        autonumber:',1)[0] if '        events: [' in ast else ''
   counts=collections.Counter(re.findall(r'^            ([A-Za-z_]+)(?:\(|,)',events,re.M));rec['versions'][rev]=dict(ast_exit=result['ast']['exit'],event_counts=dict(counts),svg_exit=result['cli']['exit'],inventory=result['cli']['inventory'])
  semantic.append(rec)
(C/'semantic-inventory.json.gz').write_bytes(gzip.compress(json.dumps(semantic).encode(),mtime=0))
summary['fresh_mechanisms']={m:dict(entries=len(rr),valid=sum(r['results']['after']['cli_comparison']['exit']!=2 for r in rr),before=dict(collections.Counter(r['results']['before']['cli_comparison']['exit'] for r in rr)),after=dict(collections.Counter(r['results']['after']['cli_comparison']['exit'] for r in rr))) for m in sorted(set(r['mechanism'] for r in fresh)) for rr in [[r for r in fresh if r['mechanism']==m]]}
# Preserve and compare exact failure names, copied immediately per revision.
summary['broad_gates']={}
for rev in ['before','after']:
 logs={x:(C/rev/(x+'.log')).read_text() for x in ['units','cli','tiers','fmt','clippy','clippy-render']}
 standing=re.findall(r'^  (test-diagrams/perturbations/[^:]+\.puml):',logs['tiers'],re.M)
 n=int(subprocess.check_output(['git','ls-tree','-r','--name-only','711dbca7' if rev=='before' else 'ae1e0de4','--','test-diagrams/perturbations'],cwd=R).decode().count('.puml\n'))
 summary['broad_gates'][rev]=dict(unit_counts=re.findall(r'test result: ok\. (\d+) passed',logs['units']),cli_counts=re.findall(r'test result: ok\. (\d+) passed',logs['cli']),ascii_tests=len(re.findall(r'^test ascii::.* \.\.\. ok$',logs['units'],re.M)),product=re.findall(r'golden_no_oracle: (\d+/\d+)',logs['tiers']),assisted=re.findall(r'golden_pairs: (.*)',logs['tiers']),standing=dict(entries=n,failed=len(standing),matched=n-len(standing)),guards='4/6',failure_names={f:sha((C/rev/f).read_bytes()) for f in ['no_oracle_failure_names.txt','golden_failure_names.txt']},clippy_render_errors=len(re.findall(r'^error:',logs['clippy-render'],re.M))-1)
 (C/rev/'standing-failure-names.json').write_text(json.dumps(standing,indent=2))
summary['failure_names_equal']={f:(C/'before'/f).read_bytes()==(C/'after'/f).read_bytes() for f in ['no_oracle_failure_names.txt','golden_failure_names.txt']}
# Preserve complete active residue and all exact source mappings, without fixing or waiving prior failure.
remaining=[dict(source=r['source'],golden=r['golden'],before=r['results']['before']['cli_comparison']['exit'],after=r['results']['after']['cli_comparison']['exit']) for r in rows if r['results']['after']['cli_comparison']['exit'] not in [0,2]]
(C/'remaining-failures.json').write_text(json.dumps(remaining,indent=2))
summary['maker_artifact_hash_changes']=[p for p,h in read(D/'artifact-sha256.json').items() if sha((D/p).read_bytes())!=h]
summary['fresh_sources_overlap_frozen']=set(r['source_sha256'] for r in fresh).intersection(r['source_sha256'] for r in rows).__len__()
(C/'summary.json').write_text(json.dumps(summary,indent=2));print('maker reproduction differences',comparisons);print('parent',summary['parent1449']);print('broad',summary['broad_gates']);print('fresh entries',len(fresh),'losses',len(summary['fresh86']['losses']['cli_comparison']),'recoveries',len(summary['fresh86']['recoveries']['cli_comparison']))
