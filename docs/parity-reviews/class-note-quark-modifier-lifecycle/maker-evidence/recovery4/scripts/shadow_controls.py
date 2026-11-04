# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib, json, os, subprocess
from pathlib import Path
import yaml
s=Path(__file__).resolve().parent
out=s/'shadow-controls';out.mkdir(exist_ok=True)
cases=[]
def add(name,mechanism,body,entities,notes=0,packages=0):
 cases.append(dict(name=name,mechanism=mechanism,source='@startuml\n'+body+'\n@enduml\n',entities=entities,notes=notes,packages=packages))
for linked in [False,True]:
 for shadow in ['false','true']:
  body=f'skinparam shadowing {shadow}\nclass Spindle {{\n+head\n+turn()\n}}'
  if linked:body+='\nclass Bearing\nSpindle --> Bearing'
  add(f'shadow_{linked}_{shadow}','degenerated-shadow-envelope',body,2 if linked else 1)
class Loader(yaml.SafeLoader):pass
Loader.add_multi_constructor('!',lambda loader,suffix,node:{'tag':suffix,'value':loader.construct_scalar(node) if isinstance(node,yaml.ScalarNode) else loader.construct_sequence(node) if isinstance(node,yaml.SequenceNode) else loader.construct_mapping(node)})
env=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
records=[]
for case in cases:
 p=out/(case['name']+'.puml');p.write_text(case.pop('source'))
 results=[]
 for flag,suffix in [('-tsvg','.rust.svg'),('--yaml','.yaml')]:
  cmd=[str(s/'target/release/rustuml'),flag,str(p)];r=subprocess.run(cmd,env=env,capture_output=True,timeout=30);p.with_suffix(suffix).write_bytes(r.stdout);p.with_suffix(suffix+'.stderr').write_bytes(r.stderr);results.append(dict(command=cmd,exit=r.returncode,stderr=r.stderr.decode()))
 model=yaml.load(p.with_suffix('.yaml').read_text(),Loader=Loader) if results[1]['exit']==0 else None
 retained=model is not None and model.get('type')=='Class' and all(len(model['diagram'][key])==case[key] for key in ['entities','notes','packages'])
 records.append(dict(**case,source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),reference_status='pending Java validation; transport unavailable; no local Java launched',model_retained=retained,runs=results))
(s/'shadow-controls.json').write_text(json.dumps(records,indent=2)+'\n')
print('cases',len(records),'Rust render successes',sum(r['runs'][0]['exit']==0 for r in records),'model retention',sum(r['model_retained'] for r in records))
