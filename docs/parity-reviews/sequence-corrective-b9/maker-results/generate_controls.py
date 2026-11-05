# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json
from pathlib import Path
cache=Path(__file__).parent;inputs=cache/'reference-inputs';inputs.mkdir(exist_ok=True)
cases=[]
def add(name,body,axes):
 p=inputs/(name+'.puml');data=('@startuml\n'+body+'@enduml\n').encode();p.write_bytes(data)
 cases.append(dict(name=name,source=str(p),source_sha256=hashlib.sha256(data).hexdigest(),timezone='GMT+11:00',cwd='PRIVATE_SEQUENCE_CORRECTIVE_B9_DIR',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_SEQUENCE_CORRECTIVE_B9_DIR','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes))
for engine in [0,1]:
 pragma='!pragma teoz true\n' if engine else ''
 add(f'color_self_{engine}',pragma+'Juniper -> Juniper ++ : begin\nreturn #transparent clear\nJuniper -> Juniper ++ : again\nreturn #39abef80 translucent\n',['engine','self return and transparent/alpha'])
 add(f'color_external_{engine}',pragma+'participant Quartz\n[-> Quartz ++ : enter\nreturn #c gray\nQuartz -> Quartz ++ : loop\nreturn #10203000 invisible alpha\n',['engine','external/self return and gray/zero alpha'])
 add(f'display_escaped_{engine}',pragma+'Harbor -> Orchard ++ : notify\nnote right : first narrow\\nsecond wider text\nreturn done\n',['engine','escaped multiline and activation'])
 add(f'display_block_{engine}',pragma+'Harbor -> Orchard ++ : notify\nnote right\nfirst narrow\nsecond wider text\nend note\nreturn done\n',['engine','physical multiline and activation'])
 for alignment,shadow in [('center',2),('right',6)]:
  add(f'style_{engine}_{alignment}',pragma+f'skinparam noteTextAlignment {alignment}\n<style>\nsequenceDiagram {{\n note {{\n Shadowing {shadow}\n }}\n}}\n</style>\nNorth -> South ++ : contact\nnote right : narrow\\na deliberately wider line\nreturn end\n',['engine','alignment/scoped numeric shadow/multiline'])
 add(f'theme_activation_{engine}',pragma+'!theme cerulean\nBirch -> Hazel ++ : start\nHazel -> Hazel ++ : nested\nnote right : themed component\nreturn inner\nreturn outer\n',['engine','theme and nested activation'])
for order in [0,1]:
 declarations='participant Wren\nparticipant Finch\n' if order==0 else 'participant Finch\nparticipant Wren\n'
 add(f'legacy_rawclose_{order}',declarations+'Wren -> Wren : establish\ndeactivate Finch\nactivate Finch\nWren -> Finch : middle\nactivate Finch\nWren -> Finch : finish\n',['participant order','unmatched close and nested reopening'])
for predecessor in ['message','note']:
 for multiline in [0,1]:
  label='parallel\\nsecond line' if multiline else 'parallel'
  prior='Ash -> Ash ++ : recur\n' if predecessor=='message' else 'note over Ash : independent note\n'
  add(f'parallel_{predecessor}_{multiline}','!pragma teoz true\nAsh -> Elm ++ : begin\n'+prior+f'& Ash -> Elm ++ : {label}\n& return inner\nElm -> Ash : after\ndeactivate Elm\n',['graphical predecessor','self/ordinary contact and multiline'])
manifest=dict(schema_version=1,purpose='Source-derived sequence corrective controls after checker-b9c6; no local Java',remote_private_directory='allocate with existing new-worker runner',cases=cases)
p=cache/'reference-manifest.json';p.write_text(json.dumps(manifest,indent=2)+'\n');print(len(cases),hashlib.sha256(p.read_bytes()).hexdigest(),p)
