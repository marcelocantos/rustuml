# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import json,hashlib
c=Path(__file__).parent;d=c/'reference-inputs';d.mkdir(exist_ok=True);cases=[]
def add(name,body,axes,engine=0):
 p=d/(name+'.puml');assert not p.exists();p.write_text('@startuml\n'+('!pragma teoz true\n' if engine else '')+body+'\n@enduml\n');cwd='PRIVATE_SEQUENCE_DECORATION_HEIGHT_DIR'
 cases.append(dict(name=name,source=str(p),source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),timezone='GMT+11:00',cwd=cwd,invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd',cwd,'--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes))
for e in [0,1]:
 add(f'circle_internal_{e}','participant Wren\nparticipant Lark\nWren -[#34567880]>o Lark ++ : forward\nWren o<-[#678934fe]- Lark : reverse\nLark -->>o Wren : open\ndeactivate Lark',['circle versus normal/open head','direction/activation/alpha'],e)
 add(f'circle_self_{e}','Wren -> Wren ++ : seed\nWren ->o Wren : nested circle\nWren o<-- Wren : reverse circle\nWren -->>o Wren -- : open close',['self circle normal/open','direction/live depth'],e)
 add(f'circle_external_{e}','participant Lark\n[->o Lark ++ : arrive\nLark o<--] : right arrive\nLark ->o] : leave\n[<--o Lark -- : left leave',['both borders','direction/activation'],e)
 add(f'circle_both_{e}','Wren o->o Lark : two circles\nLark o-->>o Wren : two circles open\nWren -/o Lark : half head',['source/target independent decorations','normal/open/half'],e)
 for n,m in [(11,13),(18,10),(10,20)]:
  add(f'child_fonts_{e}_{n}_{m}',f'skinparam noteFontSize {n}\nskinparam sequenceMessageFontSize {m}\nLark -> Wren ++ : call\nnote right : compact\\n<size:16>taller</size>\nWren -> Lark : next\ndeactivate Wren',['note/message font relation','styled multiline/activation'],e)
 add(f'child_two_notes_{e}','skinparam noteFontSize 11\nskinparam Padding 2\nWren <- Lark : reverse\nnote left : first\\nsecond\nnote right : compact\nLark -> Wren : next',['multiple children','reverse/padding/font'],e)
 add(f'style_root_{e}','<style>\nroot {\nLineColor #7854ab\n}\n</style>\nWren -> Lark ++ : enter\nnote right : inherited\nreturn complete',['root inherited line color','participant/arrow/activation/note'],e)
 add(f'style_siblings_{e}','<style>\nsequenceDiagram {\nlifeLine {\nLineColor #b23456\n}\nactivationBox {\nLineColor #3467b2\n}\n}\n</style>\nLark -> Wren ++ : seed\nWren -> Wren : self\nreturn complete',['lifeLine versus activationBox siblings','ordinary/self/return'],e)
 add(f'style_shadow_{e}','<style>\nroot {\nShadowing 4\n}\nsequenceDiagram {\nparticipant {\nShadowing '+('6' if e else '2')+'\n}\nnote {\nShadowing 0\n}\n}\n</style>\nLark -> Wren ++ : enter\nnote right : no note shadow\nreturn complete',['root versus component numeric shadow','independent participant/note/activation'],e)
 add(f'invalid_circle_order_{e}','Wren -[#a]o> Lark : invalid decoration order',['head/decoration grammar negative','engine'],e)
p=c/'reference-manifest.json';assert not p.exists();p.write_text(json.dumps(dict(schema_version=1,purpose='Bounded sequence destination decoration, complete child allocation and component style controls; accounts0b329582',remote_private_directory='allocate with existing new-worker runner',cases=cases),indent=2)+'\n');print(len(cases),str(p),hashlib.sha256(p.read_bytes()).hexdigest())
