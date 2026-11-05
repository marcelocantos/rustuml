# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,json,re
from pathlib import Path
from replay import R,S,load,dump,sha
suites={n:load(S/(n+'-results.json')) for n in ['collateral','prior31','fresh29','maker15','maker17','maker18','fresh25','fresh4']}
sets={n:{(r['source_sha256'],r['golden_sha256']) for r in rows} for n,rows in suites.items()}
by={r['source']:r for r in suites['collateral']+suites['prior31']}
for r in suites['fresh29']:
 p=R/'test-diagrams/perturbations'/r['mechanism']/'checker5'/Path(r['source']).name
 assert p.read_bytes()==Path(r['actual_source']).read_bytes();assert p.with_suffix('.svg').read_bytes()==Path(r['actual_golden']).read_bytes();by[str(p.relative_to(R))]=r
assert len(by)==1576
failures={}
for line in (S/'logs/standing.log').read_text().splitlines():
 if line.startswith('  test-diagrams/perturbations/'):
  p,msg=line[2:].split(': ',1);failures[p]=msg
assert set(failures)<=set(by)
differences=[dict(source=p,standing_pass=p not in failures,cli_pass=r['phases']['after']['svg_match'],standing_failure=failures.get(p),cwd=r.get('cwd','.'),zone=r.get('zone','GMT+11:00'),debug=r.get('debug','date=1774210426000,tz=AEDT+1100'),env=r.get('env',{})) for p,r in by.items() if (p not in failures)!=r['phases']['after']['svg_match']]
panics=collections.defaultdict(list);errors=[];drops=[];negatives=[]
for suite,rows in suites.items():
 for r in rows:
  a,b=r['phases']['after'],r['phases']['before'];d=S/'replay'/suite/r['capture_id']/'after'
  if a['panic']:
   err=(d/'svg.stderr').read_text();signature=err[err.index('panicked at '):].split('\nnote:',1)[0].strip();panics[signature].append(r['source'])
  elif r['reference_valid'] and a['cli_exit']:errors.append(dict(suite=suite,source=r['source'],diagnostic=a['first_diagnostic']))
  if b.get('model',{}).get('entities') is not None and (a.get('model',{}).get('entities') or 0)<b['model']['entities']:drops.append(dict(source=r['source'],valid=r['reference_valid'],before=b['model'],after=a.get('model')))
  if not r['reference_valid']:negatives.append(dict(suite=suite,source=r['source'],reference_xml_valid=r['reference_xml_valid'],cli_exit=a['cli_exit'],comparison_exit=a['comparison_exit'],panic=a['panic'],stderr=(d/'svg.stderr').read_text()))
oldsets={n:s for n,s in sets.items() if not n.startswith('fresh') or n=='fresh29'}
summary=dict(candidate='4a288dd47fe925a792f7cb672ca294f6ef6fdcf3',frozen1449=dict(total=sum(r.get('cohort')=='frozen' for r in suites['collateral']),matches=sum(r['phases']['after']['svg_match'] for r in suites['collateral'] if r.get('cohort')=='frozen')),standing=dict(total=len(by),passed=len(by)-len(failures),failed=len(failures),exit=101),matching_environment_cli=sum(r['phases']['after']['svg_match'] for r in by.values()),standing_cli_differences=differences,membership_counts={n:len(rows) for n,rows in suites.items()},unique_source_reference_pairs={n:len(v) for n,v in sets.items()},pair_overlap={a+' & '+b:len(sets[a]&sets[b]) for i,a in enumerate(sets) for b in list(sets)[i+1:]},old_union_unique_pairs=len(set.union(*oldsets.values())),union_unique_pairs=len(set.union(*sets.values())),panic_groups=dict(panics),other_valid_errors=errors,model_drops=drops)
for c in ['cohort955','cohort1044']:summary[c]=dict(total=sum(bool(r.get(c)) for r in suites['collateral']),matches=sum(r['phases']['after']['svg_match'] for r in suites['collateral'] if r.get(c)))
dump(S/'analysis.json',summary);dump(S/'negative-statuses.json',negatives)
print(json.dumps({k:v for k,v in summary.items() if k not in ['standing_cli_differences','panic_groups','model_drops','pair_overlap']},indent=2));print('panic groups',[(k,len(v)) for k,v in panics.items()]);print('standing differences',len(differences))
