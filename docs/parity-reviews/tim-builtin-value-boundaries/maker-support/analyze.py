# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,os,subprocess
from pathlib import Path
import yaml
R=Path.cwd();S=Path('/tmp/rustuml-tim5');C=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3');phase='iteration1'
def dump(p,v):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,indent=2)+'\n')
def model(data):
 root={k.value:v for k,v in yaml.compose(data).value};node={k.value:v for k,v in root['diagram'].value}
 return dict(type=root['type'].value,entities=len(node['entities'].value) if 'entities' in node else None,source={k.value:v for k,v in node['meta'].value}.get('source').value if 'meta' in node else None)
rows=json.loads((S/(phase+'-collateral.json')).read_bytes());models=[];missing=[]
for row in rows:
 if row['cohort']=='frozen':continue
 p=Path(row['source']);out=S/'outputs'/phase/p.parent.name/p.stem;old=C/'outputs/candidate'/p.parent.name/p.stem
 java=C/'baseline-capture/reference'/p.parent.name/p.stem/'java.source'
 if row['reference_invalid']:continue
 if java.exists() and (out/'rust.source').exists():
  row['source_equal']=(out/'rust.source').read_bytes()==java.read_bytes()
 else:missing.append(row['source'])
 try:
  before=model((old/'rust.yaml').read_bytes());after=model((out/'rust.yaml').read_bytes())
  models.append(dict(source=row['source'],before=before,after=after,entity_drop=before['entities'] is not None and after['entities'] is not None and after['entities']<before['entities']))
 except (KeyError,ValueError,AttributeError,TypeError,yaml.YAMLError):pass
fresh=json.loads((S/(phase+'-fresh31.json')).read_bytes())
for row in fresh:
 if not row['reference_valid']:continue
 out=S/'fresh-outputs'/phase/row['id'];sub='fresh-outputs' if (C/'fresh-outputs/candidate'/row['id']).exists() else 'supplement-outputs'
 before=model((C/sub/'candidate'/row['id']/'rust.yaml').read_bytes()) if (C/sub/'candidate'/row['id']/'rust.yaml').stat().st_size else None
 after=model((out/'rust.yaml').read_bytes())
 models.append(dict(source=row['source'],before=before,after=after,entity_drop=before is not None and before['entities'] is not None and after['entities'] is not None and after['entities']<before['entities']))
dump(S/'source-models.json',models);dump(S/'collateral-source-results.json',dict(valid_diagnostic_cases=sum(r['cohort']!='frozen' and not r['reference_invalid'] for r in rows),agreements=sum(r.get('source_equal',False) for r in rows),missing=missing,entity_drops=[m for m in models if m['entity_drop']],results=[r for r in rows if r['cohort']!='frozen']))
env=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',TZ='GMT+11:00',RAYON_NUM_THREADS='4')
controls=[];by={r['source']:r for r in rows}
for old in json.loads((R/'docs/parity-reviews/tim-expression-value-snapshot/checker4-support/controls.json').read_bytes()):
 p=R/old['source'];out=S/'controls'/p.stem;out.mkdir(parents=True,exist_ok=True)
 if 'command' in old:
  cmd=[S/(phase+'-rustuml'),p];q=subprocess.run(cmd,cwd=R,env=env,capture_output=True);(out/'rust.svg').write_bytes(q.stdout);(out/'stderr').write_bytes(q.stderr)
  compare=[S/'probe','compare',p.with_suffix('.svg'),out/'rust.svg'];c=subprocess.run(compare,capture_output=True);(out/'comparison.txt').write_bytes(c.stdout+c.stderr)
  controls.append(dict(source=old['source'],argv=list(map(str,cmd)),cli_exit=q.returncode,svg_match=q.returncode==0 and c.returncode==0))
 else:
  cmd=[S/(phase+'-rustuml'),'--yaml','-pipe'];q=subprocess.run(cmd,input=p.read_bytes(),cwd=R,env=env,capture_output=True);(out/'rust.yaml').write_bytes(q.stdout);(out/'stderr').write_bytes(q.stderr)
  java=C/'baseline-capture/reference'/p.parent.name/p.stem/'java.source'
  controls.append(dict(source=old['source'],argv=list(map(str,cmd)),cli_exit=q.returncode,svg_match=by[old['source']]['svg_match'],source_equal=model(q.stdout)['source'].encode()==java.read_bytes()))
dump(S/'controls.json',controls)
# Full input/reference manifest pins all 1547 original pairs, never mutates them.
files=[]
for p in sorted(set(R/r['source'] for r in rows+fresh)):
 for f in [p,p.with_suffix('.svg')]:files.append(dict(path=str(f.relative_to(R)),bytes=f.stat().st_size,sha256=hashlib.sha256(f.read_bytes()).hexdigest()))
dump(S/'frozen-membership.json',files)
print('source agreements',sum(r.get('source_equal',False) for r in rows),'models',len(models),'entity drops',sum(m['entity_drop'] for m in models),'controls',len(controls),sum(c['svg_match'] for c in controls))
