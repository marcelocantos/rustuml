# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent; P=C/'reference-inputs'; P.mkdir(exist_ok=False)
sha=lambda b:hashlib.sha256(b).hexdigest()
cases=[]
def add(name,body,mechanism,axes,valid=True):
 for engine in ['legacy','teoz']:
  n='b4b6_'+name+'_'+engine
  source='@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+body+'@enduml\n'
  p=P/(n+'.puml');p.write_bytes(source.encode())
  cases.append(dict(name=n,source=str(p),source_sha256=sha(p.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+[engine],mechanism=mechanism,intended_valid=valid))
W='sequence-pattern2-whitespace';D='sequence-all-participant-display-consumers';S='sequence-teoz-self-note-component';N='sequence-note-over-command-domain'
base='participant Juniper\nparticipant Kestrel\nJuniper -> Kestrel: seed\n'
# Disallowed whitespace: one mandatory command boundary and one negated token boundary.
for cp in [*range(0x2000,0x200b),0x3000,0x1680,0x205f,0xfeff]:
 ch=chr(cp)
 add(f'ws_{cp:04x}_terminator',base+'rnote over Juniper\nopaque marker\nend'+ch+'rnote\n',W,['Unicode codepoint '+hex(cp),'standalone terminator'],False)
 add(f'ws_{cp:04x}_quoted_name','participant "Ju'+ch+'niper" as J\nparticipant K\nJ -> K: ping\n',W,['Unicode codepoint '+hex(cp),'quoted participant token preserves content'])
# Explicit ASCII versus NBSP at each command family; FF/VT are byte-exact.
for tag,ch in [('nbsp','\u00a0'),('vt','\x0b'),('ff','\x0c')]:
 commands={
 'declare':f'participant{ch}"Third Finch"{ch}as{ch}F <<role>> #pink\nF -> Kestrel: ping\n',
 'activation':f'activate{ch}Kestrel #pink\nKestrel -> Juniper: reply\ndeactivate{ch}Kestrel\n',
 'note':f'hnote{ch}over{ch}Juniper,{ch}Kestrel: witness\n',
 'groups':f'alt{ch}first\nJuniper -> Kestrel: one\nelse{ch}second\nloop{ch}twice\nKestrel -> Juniper: two\nend\nend\n',
 'autonumber':f'autonumber{ch}13{ch}7\nJuniper -> Kestrel: numbered\n',
 'autonumber_resume':f'autonumber 5\nJuniper -> Kestrel: first\nautonumber stop\nautonumber{ch}resume{ch}3\nJuniper -> Kestrel: resumed\n',
 'divider':f'=={ch}boundary{ch}==\n',
 'delay':f'...{ch}waiting{ch}...\n',
 'ref':f'ref{ch}over{ch}Juniper,{ch}Kestrel: opaque\n',
 'box':f'box{ch}"Aviary" #pink\nparticipant F\nend box\nF -> Juniper: hello\n',
 'return':f'Juniper -> Kestrel ++: invoke\nreturn #blue{ch}done\n',
 'create_destroy':f'create{ch}actor "Late Finch" as F\nJuniper -> F: arrive\ndestroy{ch}F\n',
 'hide':f'hide{ch}stereotype\nactor F <<wide role>>\nF -> Juniper: call\n',
 'across':f'note{ch}across: span\n',
 'ref_multiline':f'ref{ch}over{ch}Juniper,{ch}Kestrel\nopaque\nend ref\n',
 }
 for cls,command in commands.items():add('ws_'+tag+'_'+cls,base+command,W,[tag,'command '+cls])
 # Arrow variants sampled as one isolated message each, including external forms.
 arrows=['->','-->','<-','<--','<->','<-->','->>','<<-','-\\','-/','\\-','/-','o->','->o','x->','->x','o<->o','-[#red]>','-[dashed]>','[->','->]','?->','->?']
 for i,arrow in enumerate(arrows):
  line=(arrow+ch+'Juniper' if arrow.startswith(('[','?')) else 'Juniper'+ch+arrow if arrow.endswith((']','?')) else 'Juniper'+ch+arrow+ch+'Kestrel')+': arrow\n'
  add(f'ws_{tag}_arrow_{i:02d}',base+line,W,[tag,'arrow '+arrow])
 # Negative class color token boundary, contrasting Unicode nonspace.
 add('ws_'+tag+'_color_boundary',base+f'actor Finch #pink{ch}order 4\nFinch -> Juniper: tint\n',W,[tag,'negated color token class'])
# Complete Display, including independent declaration order, final filtering and lifecycle context.
for kind in ['participant','actor','boundary','control','entity','database','collections','queue']:
 for variant in ['top','bottom','hidden','nofoot','boxed','nested','created','destroyed']:
  pos='bottom' if variant in ['bottom','nofoot','created','nested'] else 'top'
  decl=f'{kind} "<b>Silver Meadow</b>\\nthird register" as M <<s>> #LightBlue\n{kind} "Q" as Q <<longer classification>> #pink\n'
  pre=f'skinparam stereotypePosition {pos}\n'
  if variant=='nofoot':pre+='hide footbox\n'
  if variant=='hidden':pre+='hide stereotype\n'
  if variant=='boxed':decl='box "Enclosure" #ivory\n'+decl+'end box\n'
  if variant=='nested':decl='box "Outer"\nbox "Inner"\n'+decl+'end box\nend box\n'
  if variant=='created':decl='participant Sender\ncreate '+decl+'Sender -> M: born\n'
  tail='M -> Q: register\n'+('destroy M\n' if variant=='destroyed' else '')
  add(f'display_{kind}_{variant}',pre+decl+tail,D,['kind '+kind,'unequal stereotype widths','styled multiline alias and colours','context '+variant,'position '+pos])
# Self-note consumers: attachment, multi-line, owner location, nested controls, and return lowering.
for shape in ['rnote','hnote']:
 for side in ['left','right']:
  for context in ['first','last','group','return','over','nonself']:
   pre='participant Juniper\nparticipant Kestrel\nparticipant Lark\n'
   owner='Lark' if context=='last' else 'Juniper'
   msg=f'{owner} -> {owner}: recursive\n'
   if context=='return':msg=f'{owner} -> {owner} ++: recursive\nreturn yielded\n'
   if context=='nonself':msg='Juniper -> Lark: cross\n'
   note=f'{shape} '+('over '+owner if context=='over' else side)+'\n<b>amber</b>\nline two\nline three\nend note\n'
   section=msg+note
   if context=='group':section='alt enabled\nloop again\n'+section+'end\nelse disabled\nKestrel -> Lark: bypass\nend\n'
   add(f'self_{shape}_{side}_{context}',pre+section+'Juniper -> Kestrel: final\n',S,[shape,side,'owner context '+context,'multiline style'])
# Java factory alternatives: named aliases/quoted identities, no-space separators,
# stereotype/color/url options, across misspelling and / or & command modifiers.
forms={
 'single':'note over Juniper: prose\n','pair_spaced':'note over Juniper, Kestrel: prose\n','pair_tight':'note over Juniper,Kestrel: prose\n',
 'quoted':'note over "Juniper": prose\n','alias':'note over J: prose\n',
 'multiline':'note over Juniper\nfirst\nsecond\nend note\n',
 'rectangle':'rnote over Juniper,Kestrel: prose\n','hex':'hnote over Juniper: prose\n',
 'colour':'note over Juniper #pink: prose\n','shape_stereo':'rnote <<hint>> over Juniper #pink: prose\n',
 'post_stereo':'note over Juniper <<hint>>: prose\n','url':'note over Juniper [[https://example.test/n]]: prose\n',
 'across':'note across: prose\n','accross':'note accross: prose\n','across_rect':'rnote across #pink: prose\n','across_hex':'hnote across: prose\n',
 'across_multiline':'note across\nfirst\nsecond\nend note\n',
 'across_stereo':'note <<hint>> across: prose\n','across_url':'note across [[https://example.test/n]]: prose\n',
 'attached_left':'rnote left: prose\n','attached_right':'hnote right\nfirst\nsecond\nend note\n',
 'bare_coloured':'note over #pink: prohibited\n','bare_rect':'rnote over: prohibited\n','bare_hex_multiline':'hnote over\nprohibited\nend note\n'}
for name,note in forms.items():
 pre='participant "Juniper" as J\nparticipant Kestrel\nJ -> Kestrel: arrive\n' if name=='alias' else base
 add('domain_'+name,pre+note,N,['factory alternative '+name,'preceding ordinary arrow'],not name.startswith('bare'))
manifest=dict(schema_version=1,purpose='Independent b4b6 adversarial source-guided consumer-contract review',cases=cases)
raw=(json.dumps(manifest,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw)
bundle=dict(manifest=manifest,manifest_sha256=sha(raw),files=[dict(path=p.name,sha256=sha(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(P.glob('*.puml'))],resources=[])
z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z)
print(json.dumps(dict(cases=len(cases),manifest_sha256=sha(raw),bundle_sha256=sha(z)),indent=2))
