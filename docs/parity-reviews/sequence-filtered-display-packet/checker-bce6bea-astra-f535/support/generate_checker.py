# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent/'independent'; D=C/'reference-inputs';D.mkdir(exist_ok=False)
sha=lambda b:hashlib.sha256(b).hexdigest(); cases=[]
def add(mech,name,body,axes,valid=True):
 for engine in ['legacy','teoz']:
  n='indf535_'+name+'_'+engine;p=D/(n+'.puml');p.write_text('@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+body+'\n@enduml\n')
  cases.append(dict(name=n,mechanism=mech,source=str(p),source_sha256=sha(p.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+[engine],intended_valid=valid))
dis='sequence-filtered-ordered-participant-display';note='sequence-note-aggregate-paint-height';owner='sequence-external-note-owner';end='sequence-note-shape-terminator'
kinds=['participant','actor','boundary','control','entity','database','collections','queue']
for i,kind in enumerate(kinds):
 name='Cedar'+str(i);label='EnormouslyLongCedarName' if i%2 else 'Fir';st='s' if i%2 else 'ExtremelyWideTransportRole'
 add(dis,'hidden_'+kind,f'hide stereotype\n{kind} "{label}" as {name} <<{st}>>\nparticipant Wren\nshow stereotypes\nWren -> {name}: seed\nhide stereotypes', ['all-kind visibility toggle','wide-name' if i%2 else 'wide-stereotype','hide before and after'])
 add(dis,'bottom_'+kind,f'skinparam stereotypePosition bottom\n{kind} "{label}" as {name} <<{st}>>\nskinparam stereotypePosition top\nparticipant Wren\n{name} -> Wren: export',['all-kind bottom order','declaration-time skinparam','head and tail'])
add(dis,'mixed_order_restore','hide stereotype\nskinparam stereotypePosition bottom\ncollections "Hazel\\nbranch" as H <<bundle>>\nskinparam stereotypePosition top\nboundary Reed <<perimeter>>\nskinparam stereotypePosition bottom\nqueue Cove <<dispatch>>\nshow stereotype\nCove -> H: arrange\nH -> Reed: relay',['three distinct kinds','alternating declaration order','show final state','multiline names'])
add(dis,'create_box','skinparam stereotypePosition bottom\nbox Canopy\nparticipant Kestrel\nend box\nbox Understory\ncreate queue Spruce <<芽>>\nend box\nKestrel -> Spruce: sprout\nhide stereotype',['created queue','two boxes','bottom and hidden','Unicode stereotype'])
add(dis,'created_actor','skinparam stereotypePosition bottom\nparticipant Juniper\ncreate actor "FernWithLongName" as Fern <<tiny>>\nJuniper -> Fern: recruit',['created actor','bottom order','wide name'])
add(dis,'hide_selector','queue Nectar <<vastlyexpandedclassification>>\nentity Petal <<thin>>\nhide Nectar stereotype\nNectar -> Petal: dust',['selector ignored globally in Java','entity and queue'])
add(dis,'nested_hidden','hide stereotype\nbox Outer\nbox Inner\ncollections Corm <<secret>>\nend box\ndatabase Bulb <<another>>\nend box\nCorm -> Bulb: store',['nested boxes','hide before declarations','collections database'])
for shape in ['note','rnote','hnote']:
 for count in [1,2,3,5,7]:
  placement={1:'over',2:'left of',3:'right of',5:'over',7:'over'}[count];who='Larch,Alder' if count==5 else 'Larch'
  body='participant Larch\nparticipant Alder\nLarch -> Larch: inspect\n'+shape+' '+placement+' '+who+'\n'+'\n'.join(f'<size:{13+(j%3)*3}>row{j}twig</size>' for j in range(count))+'\nend note\nLarch -> Alder: resume'
  add(note,f'{shape}_{count}_rows',body,[shape,str(count)+' lines',placement,'across two' if count==5 else 'one participant','mixed font sizes','after self message'])
 for name,sizes in [('exact',[23,24,25,26,30]),('below',[24,25,26,27,28,31,34])]:
  add(note,f'{shape}_boundary_{name}','participant Acorn\n'+shape+' over Acorn\n'+'\n'.join(f'<size:{s}>leaf{j}</size>' for j,s in enumerate(sizes))+'\nend note\nAcorn -> Acorn: shake',[shape,'aggregate fractional boundary '+name,'mixed size sum '+str(sum(sizes))])
 add(note,f'{shape}_self_attached','participant Moss\nMoss -> Moss: spiral\n'+shape+' right\n<size:23>dew</size>\n<size:17>bark</size>\nspore\nend note',['attached self message',shape,'mixed size three lines'])
for tag,arrow,pos in [('left_in','[-> Rowan','right'),('left_out','[<- Rowan','left'),('right_out','Rowan ->]','right'),('right_in','Rowan <-]','left')]:
 add(owner,tag,'participant Rowan\nparticipant Linden\n'+arrow+': border\nnote '+pos+'\nannotation one\n<size:18>annotation two</size>\nend note',['external '+tag,pos,'multiline bare note'])
add(owner,'consecutive','participant Nettle\nparticipant Sorrel\n[-> Nettle: first\nSorrel ->]: second\n[<- Nettle: third\nnote right: latest three\nNettle -> Sorrel: ordinary\nnote left: ordinary four',['three consecutive external messages','reversed normalization','ordinary replaces owner','two note labels'])
add(owner,'ordinary_before','participant Tansy\nparticipant Yarrow\nTansy -> Yarrow: prior\nYarrow <-]: current\nnote left: must belong current',['stale ordinary owner','incoming right border'])
add(owner,'group_nested','participant Clover\nparticipant Lotus\ngroup Bed\nloop water\n[-> Clover: enter\nnote right: petal water\nend\nLotus ->]: leave\nnote left\nmud\nroot\nend note\nend',['nested loop group','two external owners','mixed inline multiline'])
add(owner,'over_named','participant Heather\nHeather ->]: pollen\nnote over Heather\nseed\nstem\nend note',['outgoing border','explicit over owner','standalone multiline control'])
add(owner,'over_bare','participant Thistle\n[-> Thistle: frost\nnote over: ice',['incoming border','bare over syntax'],False)
terminators=[('mismatch','rnote','end hnote',True),('upper','hnote','END RNOTE',True),('tab','note','end\trnote',True),('no_gap','hnote','endnote',True),('spaces','rnote','  end rnote  ',True),('double_gap','rnote','end  rnote',False),('suffix','hnote','end hnote extra',False)]
for tag,shape,term,valid in terminators:
 add(end,'terminator_'+tag,'participant Willow\n'+shape+' over Willow\nreed\n'+term+'\nWillow -> Willow: outside',['standalone '+shape,tag,'following message'],valid)
add(end,'adjacent_group','participant Sedge\ngroup Marsh\nrnote left of Sedge\nfirst\nend hnote\nhnote right of Sedge\nsecond\nEND NOTE\nend\nSedge -> Sedge: outside',['adjacent mixed shapes','group','mismatched terminators'])
add(end,'nested_literal','participant Rush\nrnote over Rush\nouter\nhnote over Rush\ninner\nend rnote\nRush -> Rush: outside',['nested-looking text inside note','first legal terminator','following message'])
add(end,'attached_reject','participant Bracken\nBracken -> Bracken: turn\nrnote left\nwrapped\nend rnote',['attached rectangle','standalone terminator forbidden'],False)
add(end,'attached_upper','participant Bramble\nBramble -> Bramble: turn\nhnote right\nwrapped\nEND NOTE',['attached hexagon','uppercase legal terminator'])
m=dict(schema_version=1,purpose='Independent adversarial filtered Display and note consumers',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw)
b=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=p.name,sha256=sha(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(D.glob('*.puml'))],resources=[])
z=gzip.compress(json.dumps(b).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z)
for p in [C/'reference-manifest.json',C/'reference-input-bundle.json.gz',*D.iterdir()]:p.chmod(0o444)
print(json.dumps(dict(cases=len(cases),manifest_sha256=sha(raw),bundle_sha256=sha(z)),indent=2))
