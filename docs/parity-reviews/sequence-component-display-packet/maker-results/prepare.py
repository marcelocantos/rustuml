# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,shutil
from pathlib import Path
C=Path(__file__).parent;ROOT=Path.cwd();h=lambda b:hashlib.sha256(b).hexdigest()
old=ROOT/'docs/parity-reviews/sequence-composed-components-packet/maker-results'
prior=json.loads((old/'all-inputs.json').read_text());fresh=json.loads((ROOT/'docs/parity-reviews/sequence-composed-components-packet/checker-672-astra/registration.json').read_text());fresh=[dict(source=x['source'],golden=x['golden'],cohort='checker42',axes=x['axes']) for x in fresh]
assert len(prior)==2021 and len(fresh)==42
(C/'all2063.json').write_text(json.dumps(prior+fresh,indent=2)+'\n');(C/'checker42.json').write_text(json.dumps(fresh,indent=2)+'\n')
(C/'before-bin').mkdir(exist_ok=True)
for p in Path('/Users/marcelo/Library/Caches/rustuml-sequence-corrective-092/stable-bin').iterdir():shutil.copy2(p,C/'before-bin'/p.name)
shutil.copy2(old/'stable-binaries.json',C/'before-binding.json')
inputs=C/'reference-inputs';inputs.mkdir(exist_ok=True)
controls=[
('dressing_mixed',r'''participant North
participant South
North o<<--/o South: lower target
South o\\-->>o North: upper source token
North o<--\\o South: async target
''',['mixed normal/open parts','direction and circles']),
('dressing_self',r'''participant Harbor
Harbor o<<--/o Harbor: self lower
Harbor o\\-->>o Harbor ++: self open
Harbor o<--\\o Harbor --: self close
''',['self source/target dressing','activation and mixed heads']),
('dressing_external',r'''participant Harbor
[<-o Harbor: reverse border
Harbor o/--]: upper border
[->>o Harbor ++: open entry
Harbor o\\--] --: thin exit
''',['external reversed direction','normal/open half and activation']),
('create_live',r'''<style>
sequenceDiagram {
 actor {
 Shadowing 4
 }
}
</style>
participant Ridge
create actor Valley
Ridge -> Valley ++: born
Valley -> Ridge: reply
Ridge -> Valley --: close
''',['created actor full image','live adjustment and scoped shadow']),
('create_repeat',r'''actor Vale
participant Hill
create Vale
Hill -> Vale: first
Vale -> Hill: work
create Vale
Hill -> Vale ++: second
Vale -> Hill --: release
create Vale
Hill -> Vale: third
''',['repeated creation ownership','reverse topology and activation']),
('queue_display',r'''skinparam participantFontSize 17
participant Gate
create queue "Long\nShort\nLast" as Mail
Gate -> Mail: enqueue
Mail --> Gate: reply
''',['three-line Display','created queue and font variation']),
('queue_stereotype',r'''box Couriers
queue "Entry\nSecond" as Mail <<parcel>>
participant Gate
end box
create Mail
Gate -> Mail ++: enqueue
Mail --> Gate --: reply
destroy Mail
''',['boxed stereotype plus multiline label','creation and destroy']),
('head_display',r'''<style>
sequenceDiagram {
 participant {
 Shadowing 6
 }
 actor {
 Shadowing 0
 }
}
</style>
actor Watcher
participant "Long\nShort\nTail" as Desk
Watcher -> Desk: check
''',['multiline head and shadow sibling','actor/participant bottom alignment']),
('rule_space',r'''skinparam noteFontSize 16
skinparam noteTextAlignment right
participant Uno
participant Dos
Uno -> Dos: begin
note right
==   ==
body
..  ..
next
__ title __
end note
''',['whitespace-only versus nonempty titles','font/alignment and sections']),
('unclosed_note',r'''participant Uno
participant Dos
Uno -> Dos: begin
note over Uno, Dos
unclosed
''',['over-participant note syntax','end-of-input closure'])]
cases=[]
for name,body,axes in controls:
 for teoz in [False,True]:
  n=name+('_teoz' if teoz else '_legacy');s=('@startuml\n'+('!pragma teoz true\n' if teoz else '')+body+'@enduml\n').encode();p=inputs/(n+'.puml');assert not p.exists();p.write_bytes(s)
  cases.append(dict(name=n,source=str(p),source_sha256=h(s),timezone='GMT+11:00',cwd='PRIVATE_CWD',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+['teoz' if teoz else 'legacy'],intended_valid=name!='unclosed_note'))
m=dict(schema_version=1,purpose='Source-guided component controls; preimplementation account bf400d84',remote_private_directory='Parent existing runner PRIVATE_CWD substitution only',cases=cases);b=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(b)
files=[dict(path=p.name,sha256=h(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(inputs.glob('*.puml'))]
bundle=dict(manifest=m,manifest_sha256=h(b),files=files,resources=[]);z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z)
print(json.dumps(dict(manifest_sha256=h(b),gzip_sha256=h(z),cases=len(cases)),indent=2))
