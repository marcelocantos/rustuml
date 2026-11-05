# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,hashlib,json,subprocess,xml.etree.ElementTree as ET
from pathlib import Path
R=Path.cwd();S=Path('/Users/marcelo/Library/Caches/rustuml-tim8-maker');sha=lambda b:hashlib.sha256(b).hexdigest()
def load(p):return json.loads(p.read_bytes())
def dump(p,x):p.write_text(json.dumps(x,indent=2)+'\n')
phase='iteration2';names=['collateral','prior31','fresh29','maker15','maker17','maker18','fresh6','newmaker15','supplement2','fresh23'];suites={n:load(S/(phase+'-'+n+'.json')) for n in names}
sets={n:{(r['membership']['source_sha256'],r['membership']['golden_sha256']) for r in rows} for n,rows in suites.items()}
classifications=[]
for suite,folder in [('new13','relay13-iteration2'),('new2','relay2-iteration2')]:
 rows=load(S/folder/'results.json');sets[suite]={(r['source_sha256'],sha((S/folder/Path(r['source']).stem/'java.svg.stdout').read_bytes())) for r in rows}
 for r in rows:
  d=S/folder/Path(r['source']).stem;svg=(d/'java.svg.stdout').read_bytes();src=(d/'java.source.stdout').read_bytes();root=ET.fromstring(svg);texts=[n.text or '' for n in root.iter() if n.tag.endswith('text')];welcome='Welcome to PlantUML!' in texts
  category='welcome_after_empty_sub_truncation' if welcome and b'@enduml' not in src else 'valid_class' if r['reference_valid'] and root.get('data-diagram-type')=='CLASS' else 'other_nonpositive'
  classifications.append(dict(source=r['source'],intended_validity=r.get('intended_validity'),java_svg_exit=r['java_exit'],java_probe_exit=r['java_probe_exit'],xml_valid=r['xml_valid'],factory=root.get('data-diagram-type'),reference_valid=r['reference_valid'],category=category,evaluated_source=src.decode(),evaluated_source_sha256=sha(src),comparator_rejects_reference=r['measurements'][phase]['comparison_exit']==2,source_equal=r['measurements'][phase]['source_equal'],svg_match=r['measurements'][phase]['svg_match']))
# All stored perturbation memberships, plus original alias/duplicate entries.
by={r['membership']['source']:r for n in ['collateral','prior31'] for r in suites[n]}
for suite,checker in [('fresh29','checker5'),('fresh6','checker6'),('fresh23','checker7')]:
 for r in suites[suite]:
  m=r['membership'];p=R/'test-diagrams/perturbations'/m['mechanism']/checker/Path(m['source']).name
  assert sha(p.read_bytes())==m['source_sha256'];assert sha(p.with_suffix('.svg').read_bytes())==m['golden_sha256'];by[str(p.relative_to(R))]=r
assert len(by)==1628
failures={}
for line in (S/'logs/standing.log').read_text().splitlines():
 if line.startswith('  test-diagrams/perturbations/'):
  p,msg=line[2:].split(': ',1);failures[p]=msg
assert set(failures)<=set(by)
differences=[dict(source=p,standing_pass=p not in failures,cli_pass=r['phases'][phase]['svg_match'],standing_failure=failures.get(p),env=r['env_overrides'],cwd=r['membership'].get('cwd','.')) for p,r in by.items() if (p not in failures)!=r['phases'][phase]['svg_match']]
panics=collections.defaultdict(list);errors=[];drops=[];source_mismatches=[];negatives=[];gains=[];losses=[];newerrors=[]
for suite,rows in suites.items():
 for r in rows:
  a,b=r['phases'][phase],r['before'];m=r['membership'];d=S/'replay'/phase/suite/f"{r['index']:04}-{Path(r['input']).stem}";err=(d/phase/'svg.stderr').read_text()
  if a['panic']:panics[err[err.index('panicked at '):].split('\nnote:',1)[0].strip()].append(r['input'])
  elif m['reference_valid'] and a['svg']['exit']:errors.append(r['input'])
  if (b.get('model') or {}).get('entities') is not None and ((a.get('model') or {}).get('entities') or 0)<b['model']['entities']:drops.append(dict(source=r['input'],valid=m['reference_valid'],before=b['model'],current=a.get('model')))
  if m['reference_valid'] and 'reference_source' in m and not a.get('exact_source_equal',False):source_mismatches.append(r['input'])
  if not m['reference_valid']:negatives.append(dict(suite=suite,source=r['input'],classification=r.get('classification'),cli_exit=a['svg']['exit'],panic=a['panic'],stderr=err,exact_source_equal=a.get('exact_source_equal')))
  if a['svg_match'] and not b['svg_match']:gains.append(r['input'])
  if b['svg_match'] and not a['svg_match']:losses.append(r['input'])
  if m['reference_valid'] and not b['svg']['exit'] and a['svg']['exit']:newerrors.append(r['input'])
summary=dict(implementation=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),measurement_phase=phase,module_sha256=sha((R/'crates/rustuml-parser/src/preprocess/mod.rs').read_bytes()),frozen_subsets={str(n):dict(total=sum(bool(r['membership'].get(k)) if k!='cohort' else r['membership'].get(k)=='frozen' for r in suites['collateral']),matches=sum(r['phases'][phase]['svg_match'] for r in suites['collateral'] if (r['membership'].get(k)=='frozen' if k=='cohort' else r['membership'].get(k)))) for n,k in [(955,'cohort955'),(1044,'cohort1044'),(1449,'cohort')]},standing=dict(total=len(by),passed=len(by)-len(failures),failed=len(failures),exit=101),matching_environment_cli=sum(r['phases'][phase]['svg_match'] for r in by.values()),standing_cli_differences=differences,membership_counts={n:len(rows) for n,rows in suites.items()}|{'new13':13,'new2':2},unique_source_reference_pairs={n:len(v) for n,v in sets.items()},pair_overlap={a+' & '+b:len(sets[a]&sets[b]) for i,a in enumerate(sets) for b in list(sets)[i+1:]},union_unique_pairs=len(set.union(*sets.values())),panic_groups=dict(panics),other_valid_errors=errors,model_drops=drops,valid_source_mismatches=source_mismatches,retained_and_checker7_gains=gains,lost_valid_passes=losses,new_valid_errors=newerrors)
dump(S/'analysis.json',summary);dump(S/'negative-statuses.json',negatives);dump(S/'new-reference-classification.json',classifications)
J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');rev='71806a23780b04a5ccde2f8ceb5121edad5eb711';paths=['tim/TrieImpl.java','tim/TMemoryGlobal.java','tim/TMemoryLocal.java','tim/VariableManager.java','tim/TContext.java','tim/TFunctionImpl.java','tim/Eater.java','tim/builtin/SetVariableValue.java','text/TLineType.java','tim/iterator/CodeIteratorSub.java','tim/iterator/CodeIteratorImpl.java','tim/iterator/CodeIteratorReturnFunction.java','tim/iterator/CodeIteratorProcedure.java','tim/TimLoader.java','BlockUml.java','PSystemBuilder.java','eggs/PSystemWelcomeFactory.java'];prov=[]
for p in paths:
 full='src/main/java/net/sourceforge/plantuml/'+p;b=(J/full).read_bytes();assert b==subprocess.check_output(['git','-C',str(J),'show',rev+':'+full]);prov.append(dict(path=full,revision=rev,sha256=sha(b)))
dump(S/'java-source-provenance.json',prov)
print(json.dumps({k:v for k,v in summary.items() if k not in ['standing_cli_differences','panic_groups','model_drops','pair_overlap']},indent=2))
