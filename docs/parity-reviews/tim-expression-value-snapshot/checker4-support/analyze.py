# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,subprocess,tarfile,xml.etree.ElementTree as ET
from pathlib import Path
import yaml
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-cb14b5f3');S=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3');P=R/'docs/parity-reviews/tim-typed-values-and-binding/checker3-support'
rows=json.loads((S/'collateral-results.json').read_text());models=[]
def fields(data):
 node=yaml.compose(data)
 return {k.value:v for k,v in node.value} if node else {}
def model(data):
 root=fields(data);node={k.value:v for k,v in root['diagram'].value}
 return dict(type=root['type'].value,entities=len(node['entities'].value) if 'entities' in node else None,source={k.value:v for k,v in node['meta'].value}.get('source').value if 'meta' in node else None)
with tarfile.open(P/'measurement-artifacts.tar.gz') as t:
 for row in rows:
  if row['cohort']=='frozen':continue
  p=Path(row['source']);a=S/'outputs/candidate'/p.parent.name/p.stem
  j=S/'baseline-capture/reference'/p.parent.name/p.stem/'java.source'
  if j.exists() and (a/'rust.source').exists():row['source_equal']=(a/'rust.source').read_bytes()==j.read_bytes() and not row['reference_invalid']
  old='outputs/after/'+p.parent.name+'/'+p.stem+'/rust.yaml'
  if row['reference_invalid']:continue
  try:
   b=model(t.extractfile(old).read());after=model((a/'rust.yaml').read_bytes());models.append(dict(source=row['source'],before=b,after=after,entity_drop=b['entities'] is not None and after['entities'] is not None and after['entities']<b['entities']))
  except (KeyError,TypeError,ValueError,AttributeError,yaml.YAMLError):pass
(S/'collateral-results.json').write_text(json.dumps(rows,indent=2)+'\n')
(S/'collateral-source-models.json').write_text(json.dumps(models,indent=2)+'\n')
summary=json.loads((S/'collateral-summary.json').read_text());summary['after']['sources']=sum(r.get('source_equal',False) for r in rows)
for k in ['fresh','retained','supplement']:summary['cohorts'][k]['after']['sources']=sum(r.get('source_equal',False) for r in rows if r['cohort']==k)
(S/'collateral-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
# Membership overlaps are explicit; none are additive denominators.
maker=R/'docs/parity-reviews/tim-expression-value-snapshot/maker-support'
local=set()
for f in ['final-frozen.json','final-fresh.json','final-retained.json','final-supplement.json']:local.update(r['source'] for r in json.loads((maker/f).read_text()))
full={r['source'] for r in rows};membership=dict(full=1516,parent=1449,checker3_additional=67,maker_available=len(local),intersection=len(full&local),only_full=sorted(full-local),only_maker=sorted(local-full),cohort955_subset_of1044=True,cohort1044_subset_of1449=True)
assert len(full-local)==411 and not local-full
(S/'membership.json').write_text(json.dumps(membership,indent=2)+'\n')
# Panic entries grouped by failure location and message, excluding process IDs.
panics={}
for row in rows:
 if not row['panic']:continue
 p=Path(row['source']);stderr=(S/'outputs/candidate'/p.parent.name/p.stem/'rust.stderr').read_text();key=stderr[stderr.index('panicked at '):].split('\nnote:')[0].strip();panics.setdefault(key,[]).append(row['source'])
(S/'panic-groups.json').write_text(json.dumps(panics,indent=2)+'\n')
# Five class controls were already in the measured union; six raw controls also get file-mode execution.
controls=[];env=dict(__import__('os').environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
for row in json.loads((maker/'file-reader-results.json').read_text()):
 p=R/row['source'];dest=S/'file-controls'/p.stem;dest.mkdir(parents=True,exist_ok=True)
 cmd=[str(S/'candidate-rustuml'),str(p)];out=subprocess.run(cmd,cwd=R,env=env,capture_output=True);(dest/'rust.svg').write_bytes(out.stdout);(dest/'stderr').write_bytes(out.stderr)
 cmp=subprocess.run([str(S/'probe'),'compare',str(p.with_suffix('.svg')),str(dest/'rust.svg')],capture_output=True);(dest/'comparison.txt').write_bytes(cmp.stdout+cmp.stderr)
 controls.append(dict(source=row['source'],command=cmd,cli_exit=out.returncode,svg_match=out.returncode==0 and cmp.returncode==0))
by={r['source']:r for r in rows}
for row in json.loads((maker/'class-tim-controls.json').read_text()):
 p=Path(row['source']);src=S/'outputs/candidate'/p.parent.name/p.stem/'rust.source'
 # These frozen inputs did not need YAML in the collateral loop; one targeted diagnostic now.
 out=subprocess.run([str(S/'candidate-rustuml'),'--yaml','-pipe'],input=(R/p).read_bytes(),cwd=R,env=env,capture_output=True)
 dest=S/'class-controls'/p.stem;dest.mkdir(parents=True,exist_ok=True);(dest/'rust.yaml').write_bytes(out.stdout);(dest/'stderr').write_bytes(out.stderr)
 j=S/'baseline-capture/reference'/p.parent.name/p.stem/'java.source'
 controls.append(dict(source=str(p),cli_exit=out.returncode,svg_match=by[str(p)]['svg_match'],source_equal=j.exists() and model(out.stdout)['source'].encode()==j.read_bytes()))
(S/'controls.json').write_text(json.dumps(controls,indent=2)+'\n')
print('collateral source agreements',summary['after']['sources'],'models',len(models),'drops',sum(m['entity_drop'] for m in models),'panic locations',len(panics),'controls',controls)
