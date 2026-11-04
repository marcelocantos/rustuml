# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib, json, os, subprocess
from pathlib import Path
import yaml
s=Path(__file__).resolve().parent
out=s/'fresh-final-controls';out.mkdir(exist_ok=True)
cases=[]
def add(name,mechanism,body,entities,notes=0,packages=0):
 cases.append(dict(name=name,mechanism=mechanism,source='@startuml\n'+body+'\n@enduml\n',entities=entities,notes=notes,packages=packages))
for position,closing in [('left','}'),('bottom','end note')]:
 opener=' {' if closing=='}' else ''
 add('modifier_'+position,'note-quark',f'class Meridian\nnote {position} of Meridian $audit <<notice>> #pink [[https://example.org/meridian{{context}}]]{opener}\ncomponent is body text\nclass still body text\n{closing}',1,1)
for order in ['group','class']:
 tail='package Basin {\nclass Rivet\n}' if order=='group' else 'class Rivet'
 add('quark_'+order,'note-quark',f'note "record" as Basin.Registry\n{tail}\nnote right of Basin.Registry : explanation',1,2,1)
for align in ['left','right']:
 for symbol in ['rectangle','uml2']:
  add(f'merge_{align}_{symbol}','symbol-merge',f'allowmixing\nskinparam componentStyle {symbol}\n<style>\ncomponent {{\nHorizontalAlignment {align}\n}}\n</style>\nclass Pin\ncomponent "A longer field station\\nx" as Service <<routed>>\nService --> Pin',2)
for shadow in ['false','true']:
 for direction in ['top to bottom direction','left to right direction']:
  add(f'actor_{shadow}_{direction[:4]}','actor-shadow',f'allowmixing\n{direction}\nskinparam shadowing {shadow}\nskinparam actorBorderThickness 2\nclass Rivet\nactor "Caretaker\\nStation" as Keeper <<operator>>\nKeeper --> Rivet',2)
for size in [12,18]:
 for stereo in ['', ' <<edge>>']:
  add(f'boundary_{size}_{bool(stereo)}','boundary-frontier',f'allowmixing\n<style>\nboundary {{\nFontSize {size}\n}}\n</style>\nclass Pin\nboundary "Service gate" as Gate{stereo}\nPin --> Gate',2)
for symbol,visibility in [('rectangle','+'),('node','-')]:
 for size in [9,18]:
  add(f'package_{symbol}_{size}','package-title',f'skinparam packageStyle {symbol}\nskinparam classAttributeIconSize {size}\n{visibility}package "Field station" as Basin <<zone>> {{\nclass Pin\n}}',1,0,1)
for size in [3,22]:
 for pad in [0,3]:
  add(f'member_{size}_{pad}','member-placement',f'skinparam classAttributeIconSize {size}\nskinparam padding {pad}\nstruct Fastener {{\n+head\n-shaft\n#turn()\n~seat()\n}}',1)
for ribbon in ['header','footer']:
 for text in ['\n','\nStation register\n']:
  add(f'ribbon_{ribbon}_{bool(text.strip())}','ribbon-lines',f'{ribbon}\n{text}\nend {ribbon}\nclass Fastener',1)
for padding in [0,4]:
 for radius in [9,15]:
  add(f'header_{padding}_{radius}','header-centering',f'skinparam padding {padding}\nskinparam circledCharacterRadius {radius}\nskinparam classFontSize 20\nclass "Long catalogue\\nentry" as Catalog <<(Q,#ABCDEF) ledger>>',1)
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
(s/'fresh-final-controls.json').write_text(json.dumps(records,indent=2)+'\n')
print('cases',len(records),'Rust render successes',sum(r['runs'][0]['exit']==0 for r in records),'model retention',sum(r['model_retained'] for r in records))
