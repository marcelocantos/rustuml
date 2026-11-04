# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures, gzip, json, os, subprocess
from pathlib import Path
import yaml
s=Path(__file__).resolve().parent
class Loader(yaml.SafeLoader):pass
def tag(loader,suffix,node):
 value=loader.construct_scalar(node) if isinstance(node,yaml.ScalarNode) else loader.construct_sequence(node) if isinstance(node,yaml.SequenceNode) else loader.construct_mapping(node)
 return {'tag':suffix,'value':value}
Loader.add_multi_constructor('!',tag)
env=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
paths=json.loads((s/'all1559.json').read_text());before={r['source']:r for r in json.loads(gzip.open('docs/parity-reviews/class-command-quark-note-ownership/checker-support/models.json.gz').read())['candidate']}
def one(path):
 cmd=[str(s/'final-rustuml'),'--yaml',str(Path(path).absolute())];p=subprocess.run(cmd,env=env,capture_output=True,timeout=60)
 out=s/'final-models'/Path(path).with_suffix('.yaml');out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(p.stdout)
 return dict(source=path,command=cmd,exit=p.returncode,stderr=p.stderr.decode(errors='replace'),model=yaml.load(p.stdout,Loader=Loader) if not p.returncode else None)
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:records=list(pool.map(one,paths))
with gzip.open(s/'final-models.json.gz','wt') as f:json.dump(records,f)
changes=[];dropped=[]
for r in records:
 old=before[r['source']]['model'];new=r['model']
 if old and new and old.get('type')=='Class' and new.get('type')=='Class':
  for key in ['entities','notes','packages','relationships']:
   a=old['diagram'].get(key,[]);b=new['diagram'].get(key,[])
   if len(a)!=len(b):changes.append(dict(source=r['source'],collection=key,before=len(a),after=len(b)))
   if len(b)<len(a):dropped.append(changes[-1])
   if key=='entities':
    lost={e['id'] for e in a}-{e['id'] for e in b}
    if lost:dropped.append(dict(source=r['source'],entity_identities_lost=sorted(lost)))
 elif (old or {}).get('type') != (new or {}).get('type'):
  changes.append(dict(source=r['source'],before_family=(old or {}).get('type'),after_family=(new or {}).get('type')))
summary=dict(entries=len(records),model_errors=sum(bool(r['exit']) for r in records),changes=changes,dropped_class_objects=dropped,limitation='Object census compares existing Class model collections and entity ids; reference SVG inventory and fresh explicit model tests supplement it, not a full Java AST equivalence claim.')
(s/'model-retention-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print('models',len(records),'errors',summary['model_errors'],'dropped',len(dropped),'changes',len(changes))
