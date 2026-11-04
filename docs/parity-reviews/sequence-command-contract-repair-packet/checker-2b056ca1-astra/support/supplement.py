# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent/'supplement';C.mkdir();P=C/'reference-inputs';P.mkdir();cases=[];sha=lambda b:hashlib.sha256(b).hexdigest()
def add(name,body,mechanism,axes,valid=True):
 for engine in ['legacy','teoz']:
  n='c2b0_'+name+'_'+engine;p=P/(n+'.puml');p.write_bytes(('@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+body+'@enduml\n').encode());cases.append(dict(name=n,source=str(p),source_sha256=sha(p.read_bytes()),mechanism=mechanism,cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+[engine],intended_valid=valid))
base='participant Quartz\nparticipant Vale\nQuartz -> Vale: primer\n'
# Remaining registered SequenceDiagramFactory command families; unused
# CommandSkin and abstract CommandExoArrowAny have no independent entry.
families={
 'auto_newpage':('autonewpage{w}90\nVale -> Quartz: continued\n',True),
 'ignore_newpage':('ignore{w}newpage\nnewpage\nVale -> Quartz: continued\n',False),
 'hide_unlinked':('participant Unlinked\nhide{w}unlinked\n',True),
 'footbox_old':('footbox{w}off\n',False),
 'activate_short':('Vale{w}++\nVale -> Quartz: active\nVale{w}--\n',False),
 'deactivate_short':('Quartz -> Vale ++: active\ndeactivate{w}\n',False),
 'url':('url{w}of{w}Vale{w}is{w}[[https://example.invalid/berth]]\n',True),
 'participant_multiline':('participant{w}Depot[\nCargo\nBay]\nDepot -> Vale: manifest\n',True),
 'declaration_a2':('actor{w}Depot{w}as{w}"Freighter"\nDepot -> Vale: dock\n',True),
 'declaration_a3':('actor{w}Freighter{w}as{w}Depot\nDepot -> Vale: dock\n',True),
 'declaration_a4':('actor{w}"Freighter"\n"Freighter" -> Vale: dock\n',True),
 'link_anchor':('{first} Quartz -> Vale: ping\n{last} Vale -> Quartz: pong\n{first}{w}<->{w}{last}:link\n',False),
 'exo_out':('Vale{w}->]:depart\n',False)}
for name,(body,required) in families.items():
 for sn,s in [('vt','\v'),('ff','\f')]:add('factory_'+name+'_'+sn,base+body.replace('{w}',s),'sequence-command-fox-space',[name,sn,'factory-registration completion'],not(required and sn=='vt'))
# Changed note/across/color grammar: empty bodies and rich ColorParser tokens
# are boundary cases absent from the main matrix.
for shape in ['note','rnote','hnote']:
 add('empty_'+shape,base+shape+' across:\nVale -> Quartz: after\n','sequence-command-entry-grammar',[shape,'empty inline body','following event'])
 add('rich_color_'+shape,base+shape+' over Vale #back:LightBlue: cargo\n','sequence-pattern2-class-composition',[shape,'ColorParser named channel','colon in color'])
raw=(json.dumps(dict(schema_version=1,purpose='Registered-family completeness and empty/color grammar boundaries',cases=cases),indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw)
b=dict(manifest=json.loads(raw),manifest_sha256=sha(raw),files=[dict(path=p.name,sha256=sha(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(P.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(b).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z)
oldroot=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-8eddc9af');old={sha(p.read_bytes()) for folder in ['test-diagrams/perturbations','docs/parity-reviews'] for p in (oldroot/folder).rglob('*.puml')};old|={x['source_sha256'] for x in json.loads((C.parent/'reference-manifest.json').read_text())['cases']};assert not ({x['source_sha256'] for x in cases}&old)
print(len(cases),sha(raw),sha(z))
