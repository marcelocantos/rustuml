# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib, json
from pathlib import Path

cache=Path(__file__).parent
dest=cache/'reference-inputs'; dest.mkdir(exist_ok=False)
remote='/Users/marcelo/rustuml-reference/work/sequence-review-092cf5f-astra'
arrow='sequencediagram.command.CommandArrow#getRegexConcat/#executeArg; Rose#createComponentArrow'
exo='sequencediagram.command.CommandExoArrowLeft/Right#getRegexConcat; CommandExoArrowAny#executeArg'
note='sequencediagram.command.CommandNoteOnArrow; graphic.ArrowAndNoteBox constructor; Rose#createComponentNote'
rule='cucadiagram.BodyEnhanced2#getArea; BodyEnhancedAbstract#decorate; TextBlockLineBefore#calculateDimension'
style='skin.ComponentType#getStyleSignature; Rose#createComponentParticipant/#createComponent; Style#getShadowing'
cases=[
 ('ordinary_live','participant Quartz\nparticipant Finch\nQuartz -[#42719B80]>>o Finch ++ : longer transit\nQuartz o<-- Finch : reply\ndeactivate Finch',[arrow],['head/open/filled','direction/activation/alpha']),
 ('bidir','participant Finch\nparticipant Quartz\nQuartz o<->o Finch : doubled marker',[arrow],['both endpoints','bidirectional/declaration order']),
 ('self_half','participant Quartz\nQuartz -> Quartz ++ : acquire\nQuartz -> Quartz ++ : nested\nQuartz -//o Quartz : open half\nQuartz o\\-- Quartz : reverse half\ndeactivate Quartz\ndeactivate Quartz',[arrow],['self/open half','reverse/live depth']),
 ('exo_source','participant Finch\n[o-> Finch ++ : outer source\nFinch <--o] : right source\ndeactivate Finch',[exo],['external source decoration','border/direction/activation']),
 ('exo_target','participant Finch\n[->o Finch ++ : inner target\nFinch -->>o] : outer target\n[o<-- Finch : left target\ndeactivate Finch',[exo],['external target decoration','border/open head/live depth']),
 ('both_dispatch','Quartz o-->>o Finch : transport\nFinch -> Quartz : return leg',[arrow],['both circle dispatch','open dotted head/labels']),
 ('child_small','skinparam noteFontSize 9\nskinparam sequenceMessageFontSize 23\nQuartz -> Finch ++ : wide message\nnote right : short\nFinch -> Quartz : next\ndeactivate Finch',[note],['note below message font','activation/next row']),
 ('child_multi','skinparam noteFontSize 17\nskinparam sequenceMessageFontSize 11\nskinparam Padding 3\nQuartz <- Finch : delivery\nnote left : tall\\nsecond\\nthird\nnote right : tiny\nFinch -> Quartz : continue',[note],['unequal child heights','reverse/multiple children/padding']),
 ('rule_alone','skinparam noteFontSize 16\nskinparam sequenceMessageFontSize 11\nQuartz -> Finch : __ribbon__\nnote right\n__title__\nend note\nFinch -> Quartz : plain',[rule,note],['title wrapper','font relationship/underline']),
 ('rule_body','skinparam noteFontSize 16\nskinparam sequenceMessageFontSize 11\nQuartz -> Finch : __ribbon__\nnote right\n__title__\nbody after title\nend note\nFinch -> Quartz : plain',[rule,note],['nonempty title body','font relationship/underline']),
 ('underline','skinparam sequenceMessageFontSize 19\nQuartz -> Finch : __first__\nFinch -> Quartz : inline __second__\nQuartz -> Finch : __third__\\nlast',[arrow,'klimt.creole.legacy.AtomText#calculateDimensionSlow'],['underline inline/whole','multiline/font']),
 ('style_lines','<style>\nroot {\nLineColor #476D83\n}\nsequenceDiagram {\nlifeLine {\nLineColor #AB6723\n}\nactivationBox {\nLineColor #394BA2\n}\n}\n</style>\nQuartz -> Finch ++ : inherit\nnote left : envelope\nreturn finish',[style],['root versus sibling selectors','activation/note/return']),
 ('style_shadow','<style>\nroot {\nShadowing 5\n}\nsequenceDiagram {\nparticipant {\nShadowing 7\n}\nactivationBox {\nShadowing 0\n}\nnote {\nShadowing 0\n}\n}\n</style>\nQuartz -> Finch ++ : shadowed\nnote right : unshadowed\nreturn release',[style],['numeric participant/root shadow','activation/note override']),
 ('style_create','<style>\nsequenceDiagram {\nparticipant {\nShadowing 5\n}\n}\n</style>\nparticipant Quartz\ncreate participant Finch\nQuartz -> Finch : create\nFinch -> Quartz : alive',[style,'sequencediagram.teoz.CommunicationTile#getPreferredHeight'],['creation consumer','numeric shadow/event history']),
]
rows=[]
def add(name,body,provenance,axes,expected):
    path=dest/(name+'.puml'); path.write_bytes(body.encode()); path.chmod(0o444)
    data=path.read_bytes()
    rows.append(dict(name=name,source=str(path),remote_name=path.name,remote_source=remote+'/'+path.name,source_sha256=hashlib.sha256(data).hexdigest(),bytes=len(data),axes=axes,intended_reference=expected,source_validation=provenance,invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd',remote,'--timezone','GMT+11:00','--','-pipe','-tsvg']))
for engine in ['legacy','teoz']:
    for name,body,provenance,axes in cases:
        add(name+'_'+engine,'@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+body+'\n@enduml\n',provenance,axes+[engine],'valid sequence')
for name,body in [('invalid_inner','Quartz -o> Finch : misplaced'),('invalid_exo','[--o> Finch : misplaced')]:
    add(name,'@startuml\n'+body+'\n@enduml\n',[arrow,exo],['interior decoration','ordinary/external'],'invalid')
add('empty','', ['empty pipe input control, no intended factory'],['zero bytes','pipe front end'],'empty/non-diagram')
manifest=cache/'reference-manifest.json'
manifest.write_text(json.dumps(dict(schema_version=1,remote_private_directory=remote,java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',resource_files=[],cases=rows),indent=2)+'\n');manifest.chmod(0o444)
print(len(rows), manifest, hashlib.sha256(manifest.read_bytes()).hexdigest())
