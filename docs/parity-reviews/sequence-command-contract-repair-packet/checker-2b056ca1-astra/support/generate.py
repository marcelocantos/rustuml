# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent; P=C/'reference-inputs';P.mkdir();cases=[]
sha=lambda b:hashlib.sha256(b).hexdigest()
N='sequence-required-note-identity';F='sequence-command-fox-space';E='sequence-command-entry-grammar';K='sequence-pattern2-class-composition';S='sequence-create-command-signature-scope'
def add(name,body,mechanism,axes,valid=True):
 for engine in ['legacy','teoz']:
  n='c2b0_'+name+'_'+engine;s=('@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+body+'@enduml\n').encode();p=P/(n+'.puml');p.write_bytes(s)
  cases.append(dict(name=n,source=str(p),source_sha256=sha(s),mechanism=mechanism,cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+[engine],intended_valid=valid))
# Identities: reorder declarations, opposing message direction, quoted token versus
# quoted identity containing punctuation/space, all shapes and multiline endings.
for i,alias in enumerate(['of','over','left','right','as','across','on','end']):
 shape=['note','rnote','hnote'][i%3];decl=f'participant Remote\nparticipant "Harbour {i}" as {alias}\nRemote -> {alias}: inbound\n'
 add('alias_'+alias,decl+f'{shape} over {alias}: ledger\n{alias} -> Remote: outward\n',N,['keyword identity '+alias,'reverse declaration and arrow order',shape])
 add('quoted_'+alias,decl+f'{shape} right of "{alias}"\nline one\nline two\nend note\n',N,['quoted keyword '+alias,'right prefix','multiline',shape])
for shape in ['note','rnote','hnote']:
 for pos in ['left','right']:
  add(f'{shape}_{pos}_of','participant Remote\nparticipant of\nof -> Remote: inbound\n'+f'{shape} {pos} of of: margin\n',N,['prefix versus identity',pos,shape])
 add(shape+'_pair_of','participant of2\nparticipant of\nof2 -> of: inbound\n'+f'{shape} over of, of2\npaired ledger\nend {shape}\n',N,['pair identities','declaration order','multiline',shape])
 add(shape+'_space_identity','participant "Pier @ east"\nparticipant Remote\nRemote -> "Pier @ east": inbound\n'+f'{shape} left of "Pier @ east": cargo\n',N,['quoted space/punctuation','declaration without alias',shape])
# Signature families: all internal separators changed; VT+space places its bit
# at a different position from prior controls. Explicit validity is prediction.
base='participant Quartz\nparticipant Vale\nQuartz -> Vale: seed\n'
families={
 'participant':('participant{w}Depot\nDepot -> Vale: call\n',True),
 'activation':('activate{w}Vale\nVale -> Quartz: echo\ndeactivate{w}Vale\n',True),
 'destroy':('destroy{w}Vale\n',True),
 'create_plain':('create{w}Depot\nQuartz -> Depot: born\n',True),
 'autoactivate':('autoactivate{w}on\nQuartz -> Vale: enter\nreturn{w}done\n',True),
 'autonumber':('autonumber{w}7\nQuartz -> Vale: numbered\n',False),
 'resume':('autonumber\nQuartz -> Vale: count\nautonumber stop\nautonumber{w}resume\nQuartz -> Vale: next\n',True),
 'stop':('autonumber\nQuartz -> Vale: count\nautonumber{w}stop\nQuartz -> Vale: next\n',True),
 'increment':('autonumber 1.2\nautonumber{w}inc{w}A\nQuartz -> Vale: next\n',True),
 'note_named':('note{w}over{w}Vale:ledger\n',True),
 'note_pair':('note{w}over{w}Quartz,Vale:ledger\n',True),
 'note_attached':('note{w}right:ledger\n',False),
 'note_across':('note{w}across:ledger\n',False),
 'reference':('ref{w}over{w}Quartz,Vale:ledger\n',True),
 'reference_multi':('ref{w}over{w}Quartz,Vale\nledger\nend ref\n',False),
 'group':('alt{w}branch\nQuartz -> Vale: within\nelse{w}fallback\nVale -> Quartz: other\nend\n',False),
 'box':('box{w}"Depot"\nparticipant Yard\nend box\nYard -> Quartz: inbound\n',False),
 'box_end':('box "Depot"\nparticipant Yard\nend{w}box\nYard -> Quartz: inbound\n',True),
 'hide_stereo':('hide{w}stereotype\n',True),
 'hide_footbox':('hide{w}footbox\n',True),
 'return':('Quartz -> Vale: enter\nreturn{w}done\n',False),
 'newpage':('newpage{w}Second\nVale -> Quartz: next\n',False),
 'arrow':('Quartz{w}->{w}Vale:dispatch\n',False),
 'exo_arrow':('[->{w}Vale:external\n',False),
 'divider':('=={w}Phase{w}==\n',False),
 'delay':('...{w}Waiting{w}...\n',False),
 'spacing':('{w}|||{w}\n',False),
 'skinparam':('skinparam{w}sequenceMessageAlign{w}right\nVale -> Quartz: shifted\n',True),
}
for family,(template,required) in families.items():
 for wn,w in [('vt','\v'),('ff','\f'),('vt_space','\v ')]:
  add(f'fox_{family}_{wn}',base+template.format(w=w),F,['command family '+family,wn,'global signature with optional/nested leaves'],not(required and wn=='vt'))
# Full signature survives a create tail and nested group parser state, every kind.
for i,kind in enumerate(['participant','actor','boundary','control','entity','queue','database','collections']):
 for which in ['outer','inner']:
  command=('create\v'+kind+' Depot') if which=='outer' else ('create '+kind+'\vDepot')
  body='participant Quartz\ngroup Batch\nloop retry\n'+command+'\nQuartz -> Depot: receive\nend\nend\nDepot -> Quartz: confirm\n'
  add('create_'+kind+'_'+which,body,S,['nested group and loop','kind '+kind,which+' signature separator'])
# Entries: preserve tabs/NBSP in labels too, distinguishing token-boundary trim
# from literal text and stateful resume increment.
for wn,w in [('tab','\t'),('nbsp','\u00a0'),('spaces','   ')]:
 entries={
  'box':f'box{w}"Depot terminal" #LightBlue\nparticipant Yard\nend box\nYard -> Quartz: dock\n',
  'across':f'note{w}across #LightBlue: berth\n',
  'resume':f'autonumber 8 2\nQuartz -> Vale: count\nautonumber stop\nautonumber{w}resume{w}3\nQuartz -> Vale: next\n',
  'divider':f'=={w}Phase gate{w}==\n',
  'delay':f'...{w}At berth{w}...\n',
  'space':f'{w}|||{w}\nVale -> Quartz: next\n'}
 for name,body in entries.items():add('entry_'+name+'_'+wn,base+body,E,[wn,'entry '+name,'state or literal-label spacing'])
# Composite class: colon adjacency, mixed whitespace before colon, every note
# target and shape. Java's other composite %s class is autonumber START.
for shape in ['note','rnote','hnote']:
 for target in ['over Vale','over Quartz, Vale','left','across']:
  for sn,s in [('tight',''),('mixed','\t\u00a0')]:
   add('color_'+shape+'_'+target.split()[0]+('_pair' if ',' in target else '')+'_'+sn,base+f'{shape} {target} #LightBlue{s}: cargo\n',K,[shape,target,sn+' color-colon separator'])
for i,token in enumerate(['3.7','3:7','3-7','3\t7','3\u00a07','3\v7','3\f7']):
 add('composite_autonumber_'+str(i),base+f'autonumber {token}\nQuartz -> Vale: count\nVale -> Quartz: counted\n',K,['autonumber START [^%pLN%s]','punctuation versus separator '+repr(token)])
# Inline arrow aliases are a second tail parser, but no Fox guard is applied.
for i,w in enumerate(['\v','\f']):
 add('tail_arrow_alias_'+str(i),'participant Vale\n"Depot label"'+w+'as'+w+'Dock -> Vale: introduced\nVale -> Dock: reply\n',F,['inline arrow alias tail reparse','quoted display supplies global bit',repr(w)])
# Source novelty against every preserved source at the evidence tip (maker
# checkout read-only), including old 64/148/534 matrices and all maker controls.
oldroot=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-8eddc9af')
old={sha(p.read_bytes()) for folder in ['test-diagrams/perturbations','docs/parity-reviews'] for p in (oldroot/folder).rglob('*.puml')}
assert len({x['name'] for x in cases})==len(cases)
assert not ({x['source_sha256'] for x in cases}&old)
m=dict(schema_version=1,purpose='Independent adversarial sequence command-contract review',cases=cases)
raw=(json.dumps(m,indent=2,ensure_ascii=True)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw)
bundle=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=p.name,sha256=sha(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(P.glob('*.puml'))],resources=[])
z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z)
(C/'novelty.json').write_text(json.dumps(dict(cases=len(cases),distinct=len({x['source_sha256'] for x in cases}),prior_distinct=len(old),overlap=0),indent=2)+'\n')
print(len(cases),'manifest',sha(raw),'bundle',sha(z))
