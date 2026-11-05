# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,hashlib,base64
from pathlib import Path
C=Path(__file__).parent;D=C/'reference-inputs';D.mkdir(exist_ok=False);cases=[]
def sha(b):return hashlib.sha256(b).hexdigest()
def add(mechanism,name,body,axes,valid=True):
 for engine in ['legacy','teoz']:
  n='ind78_'+name+'_'+engine;p=D/(n+'.puml');p.write_text('@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+body+'\n@enduml\n')
  cases.append(dict(name=n,mechanism=mechanism,source=str(p),source_sha256=sha(p.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+[engine],intended_valid=valid))
p='sequence-single-pending-create';decl='participant Vega\nactor Lyra\nqueue Draco\nparticipant Corvus\n'
add(p,'triple_pending',decl+'create Lyra\ncreate Draco\ncreate Corvus\nVega -> Corvus: hatch\nCorvus -> Draco: touch\nDraco -> Lyra: wake',['triple overwrite','four kinds/order'])
add(p,'interleaved',decl+'create Lyra\nVega -> Lyra: hatch\ncreate Draco\ncreate Lyra\nVega -> Lyra: again\ncreate Draco\nLyra -> Draco: mail',['consumed plus overwritten','mixed kinds'])
add(p,'return_valid',decl+'Vega -> Lyra: call\nactivate Lyra\ncreate Vega\nreturn back',['pending before return','return destination'])
add(p,'return_invalid',decl+'Vega -> Lyra: call\nactivate Lyra\ncreate Draco\nreturn back',['pending before return','wrong receiver'],False)
add(p,'activate_between',decl+'create Draco\nactivate Lyra\nVega -> Draco: hatch\ndeactivate Lyra',['unattached initial activation','pending retention'])
add(p,'deactivate_between',decl+'Vega -> Lyra: start\nactivate Lyra\ncreate Draco\ndeactivate Lyra\nVega -> Draco: hatch',['deactivation between','mixed kind'])
for name,msg in [('incoming_left','[-> Draco'),('incoming_right','Draco <-]'),('outgoing_left','[<- Draco'),('outgoing_right','Draco ->]')]:
 add(p,name,decl+'create Draco\n'+msg+': edge',['external direction',name])
add(p,'group_scope',decl+'group outer\ncreate Lyra\nalt branch\ncreate Draco\nend\nend\nVega -> Draco: hatch',['pending crosses nested group','overwrite kinds'])
add(p,'box_scope','participant Vega\nbox Domain\nactor Lyra\nqueue Draco\nend box\ncreate Lyra\ncreate Draco\nVega -> Draco: hatch',['box membership','overwritten pending'])
add(p,'numbered',decl+'autonumber 7 3\ncreate Lyra\ncreate Draco\ncreate Corvus\nVega -> Corvus: hatch\nautonumber resume\nCorvus -> Lyra: next',['autonumber','triple pending'])
for kind in ['participant','actor','queue']:
 add(p,'dangling_'+kind,'participant Vega\n'+kind+' Sol\nVega -> Vega: before\ncreate Sol',['end without consumer',kind])
add(p,'invalid_self',decl+'create Draco\nDraco -> Draco: self',['self receiver','queue'],False)
add(p,'invalid_source',decl+'create Draco\nDraco -> Vega: source',['source receiver','queue'],False)
add(p,'invalid_neither',decl+'create Draco\nVega -> Lyra: unrelated',['neither receiver','actor'],False)
q='sequence-queue-stripe-origin'
for name,label,stereo in [('rich_stereo','<size:25>Orion</size>\\n<size:9>dust</size>\\n<size:19>tail</size>',' <<dispatch>>'),('plain_no_stereo','Orion\\n<size:24>dust</size>\\nend',''),('blank_first','\\n<size:21>Orion</size>\\nend',''),('blank_stereo','\\n<size:21>Orion</size>\\nend',' <<dispatch>>')]:
 add(q,'queue_'+name,'queue "'+label+'" as Orion'+stereo+'\nparticipant Cygnus\nOrion -> Cygnus: relay',['first/late/blank stripe',name,'tails'])
add(q,'queue_created','actor Cygnus\nqueue "<size:22>Orion</size>\\n<size:8>dust</size>\\nend" as Orion <<dispatch>>\ncreate Orion\nCygnus -> Orion: relay',['created queue','stereotype','three lines'])
add(q,'queue_box','box Habitat\nqueue "<size:24>Orion</size>\\n<size:9>dust</size>" as Orion <<dispatch>>\nend box\nactor Cygnus\nOrion -> Cygnus: relay',['queue box','actor sibling','rich label'])
add(q,'queue_order','participant Cygnus\nqueue "<size:8>Orion</size>\\n<size:22>dust</size>\\n<size:10>end</size>" as Orion\nactor Phoenix\nPhoenix -> Orion: relay',['three participants','small-large-small'])
m='sequence-message-stripe-baselines';alt='<size:8>low</size>\\n<size:24>high</size>\\n<size:10>ebb</size>\\n<size:19>rise</size>'
for name,arrow in [('forward','Vega -> Lyra'),('reverse','Vega <- Lyra'),('self','Lyra -> Lyra'),('reverse_self','Lyra <- Lyra'),('external','[-> Lyra')]:
 add(m,'stripe_'+name,'participant Vega\nparticipant Lyra\n'+arrow+': '+alt,['four alternating sizes',name])
add(m,'stripe_heading','Vega -> Lyra: plain\\n= Crest\\n<size:9>trough</size>\\n== Ridge',['heading and size','four lines'])
add(m,'stripe_styles','Vega -> Lyra: <b><size:21>crest</size></b>\\n<i><size:9>hollow</size></i>\\n<u><size:18>rise</size></u>',['inline style','alternating size'])
add(m,'stripe_note_divider','Vega -> Lyra: anchor\nnote over Vega,Lyra\n<size:8>low</size>\n<size:24>high</size>\n<size:10>ebb</size>\nend note\n== <size:8>low</size>\\n<size:24>high</size>\\n<size:10>ebb</size> ==\nLyra -> Vega: end',['separate consumer control','note and divider'])
s='sequence-self-half-polygon-order'
for name,arrow in [('top','-/'),('bottom','-\\'),('reverse_top','/-'),('reverse_bottom','\\-'),('open','-//'),('circle','o-/o'),('full','->')]:
 add(s,'self_'+name,'participant Pavo\nparticipant Tucana\nparticipant Vela\nPavo '+arrow+' Pavo: first\nactivate Vela\nactivate Vela\nVela '+arrow+' Vela: last\ndeactivate Vela\ndeactivate Vela',['first and last participant','nested activation',name])
c='sequence-collections-source-clarification'
add(c,'collection_sibling','participant "Dawn" as D\ncollections "<size:20>Archive</size>\\n<size:9>leaf</size>" as A\nD -> A: store',['plain versus collections','multiline sizes'])
add(c,'collection_font','skinparam participantFontSize 18\ncollections "Archives" as A\nparticipant Dusk\nA -> Dusk: fetch',['font size','order'])
manifest=dict(schema_version=1,purpose='Independent falsification of pending CREATE / stripe packet',cases=cases)
b=json.dumps(manifest,indent=2).encode()+b'\n';(C/'reference-manifest.json').write_bytes(b)
bundle=dict(manifest=manifest,manifest_sha256=sha(b),files=[dict(path=x['name']+'.puml',sha256=x['source_sha256'],base64=base64.b64encode(Path(x['source']).read_bytes()).decode()) for x in cases],resources=[])
(C/'reference-input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(bundle).encode(),mtime=0))
print('cases',len(cases));print('manifest',sha(b));print('bundle',sha((C/'reference-input-bundle.json.gz').read_bytes()))
