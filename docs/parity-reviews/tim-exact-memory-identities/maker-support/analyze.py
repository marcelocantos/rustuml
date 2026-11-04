# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,hashlib,json,subprocess
from pathlib import Path
R=Path.cwd();S=Path('/Users/marcelo/Library/Caches/rustuml-tim7-maker');sha=lambda b:hashlib.sha256(b).hexdigest()
def load(p):return json.loads(p.read_bytes())
def dump(p,x):p.write_text(json.dumps(x,indent=2)+'\n')
suites={n:load(S/('final2-'+n+'.json')) for n in ['collateral','prior31','fresh29','maker15','maker17','maker18','fresh6']}
sets={n:{(r['source_sha256'],r['golden_sha256']) for r in rows} for n,rows in suites.items()}
for suite,folder in [('newmaker15','relay15-final2'),('supplement2','relay2-final2')]:
 rows=load(S/folder/'results.json')
 sets[suite]={(r['source_sha256'],sha((S/folder/Path(r['source']).stem/'java.svg.stdout').read_bytes())) for r in rows}
overlap={a+' & '+b:len(sets[a]&sets[b]) for i,a in enumerate(sets) for b in list(sets)[i+1:]}
# Canonical stored pair paths, retaining each original membership, even duplicate bytes.
by={r['source']:r for r in suites['collateral']+suites['prior31']}
for suite,checker in [('fresh29','checker5'),('fresh6','checker6')]:
 for r in suites[suite]:
  p=R/'test-diagrams/perturbations'/r['mechanism']/checker/Path(r['source']).name
  assert p.read_bytes()==Path(r['source']).read_bytes();assert p.with_suffix('.svg').read_bytes()==Path(r['golden']).read_bytes()
  by[str(p.relative_to(R))]=r
assert len(by)==1605
failures={}
for line in (S/'logs2/standing.log').read_text().splitlines():
 if line.startswith('  test-diagrams/perturbations/'):
  p,msg=line[2:].split(': ',1);failures[p]=msg
assert set(failures)<=set(by),set(failures)-set(by)
differences=[dict(source=p,standing_pass=p not in failures,cli_pass=r['current']['svg_match'],standing_failure=failures.get(p),cwd=r.get('cwd','.'),zone=r.get('zone','GMT+11:00'),debug=r.get('debug','date=1774210426000,tz=AEDT+1100'),env=r.get('env',{})) for p,r in by.items() if (p not in failures)!=r['current']['svg_match']]
panics=collections.defaultdict(list);errors=[];drops=[];source_mismatches=[];negatives=[]
for suite,rows in suites.items():
 for r in rows:
  a,b=r['current'],r['before'];d=S/'replay/final2'/suite/r['capture_id']
  if a['panic']:
   err=(d/'svg.stderr').read_text();signature=err[err.index('panicked at '):].split('\nnote:',1)[0].strip();panics[signature].append(r['source'])
  elif r['reference_valid'] and a['cli_exit']:errors.append(r['source'])
  if b.get('model',{}).get('entities') is not None and (a.get('model',{}).get('entities') or 0)<b['model']['entities']:drops.append(dict(source=r['source'],valid=r['reference_valid'],before=b['model'],current=a.get('model')))
  if r['reference_valid'] and 'reference_source' in r and not a.get('source_equal',False):source_mismatches.append(r['source'])
  if not r['reference_valid']:negatives.append(dict(suite=suite,source=r['source'],reference_xml_valid=r['reference_xml_valid'],cli_exit=a['cli_exit'],panic=a['panic'],stderr=(d/'svg.stderr').read_text()))
summary=dict(implementation=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),frozen1449=dict(total=sum(r.get('cohort')=='frozen' for r in suites['collateral']),matches=sum(r['current']['svg_match'] for r in suites['collateral'] if r.get('cohort')=='frozen')),standing=dict(total=len(by),passed=len(by)-len(failures),failed=len(failures),exit=101),matching_environment_cli=sum(r['current']['svg_match'] for r in by.values()),standing_cli_differences=differences,membership_counts={n:len(rows) for n,rows in suites.items()}|{'newmaker15':15,'supplement2':2},unique_source_reference_pairs={n:len(v) for n,v in sets.items()},pair_overlap=overlap,union_unique_pairs=len(set.union(*sets.values())),panic_groups=dict(panics),other_valid_errors=errors,model_drops=drops,valid_source_mismatches=source_mismatches)
dump(S/'analysis.json',summary);dump(S/'negative-statuses.json',negatives)
# Verify the source cited in the new accounts against the pinned read-only checkout.
J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');rev='71806a23780b04a5ccde2f8ceb5121edad5eb711'
paths=['tim/TMemoryLocal.java','tim/TMemoryGlobal.java','tim/VariableManager.java','tim/FunctionsSet.java','tim/TFunctionSignature.java','tim/TFunctionImpl.java','tim/TFunctionArgument.java','tim/EaterAffectation.java','tim/EaterAffectationDefine.java','tim/EaterFunctionCall.java','tim/TContext.java','tim/iterator/CodeIteratorSub.java','tim/iterator/CodeIteratorReturnFunction.java','tim/iterator/CodeIteratorProcedure.java','tim/iterator/CodeIteratorLegacyDefine.java','tim/expression/ShuntingYard.java','tim/expression/ReversePolishInterpretor.java','tim/builtin/Filename.java','tim/builtin/SetVariableValue.java','tim/builtin/GetVariableValue.java','tim/builtin/VariableExists.java']
provenance=[]
for path in paths:
 full='src/main/java/net/sourceforge/plantuml/'+path;b=(J/full).read_bytes();expected=subprocess.check_output(['git','-C',str(J),'show',rev+':'+full]);assert b==expected;provenance.append(dict(path=full,sha256=sha(b),revision=rev))
dump(S/'java-source-provenance.json',provenance)
print(json.dumps({k:v for k,v in summary.items() if k not in ['standing_cli_differences','panic_groups','model_drops','valid_source_mismatches']},indent=2))
