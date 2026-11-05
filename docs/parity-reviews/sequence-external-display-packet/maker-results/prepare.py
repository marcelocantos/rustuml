# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,shutil
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest();old=R/'docs/parity-reviews/sequence-pending-stripe-packet/maker-results';review=old.parent/'checker-ae1e0de4-astra'
rows=json.loads((old/'all-inputs.json').read_text());fresh=[]
for f in ['fresh86.json','followup10.json']:
 for r in json.loads((review/f).read_text()):
  for key in ['source','golden']:
   p=Path('test-diagrams/perturbations')/r['mechanism']/Path(r[key]).name
   assert p.exists()
   if Path(r[key]).exists():assert p.read_bytes()==Path(r[key]).read_bytes()
   r[key]=str(p)
  fresh.append(r)
assert len(rows)==2169 and len(fresh)==96
(C/'all2265.json').write_text(json.dumps(rows+fresh,indent=2)+'\n');(C/'checker96.json').write_text(json.dumps(fresh,indent=2)+'\n')
(C/'before-bin').mkdir()
for p in Path('/Users/marcelo/Library/Caches/rustuml-sequence-corrective-711/final-bin').iterdir():shutil.copy2(p,C/'before-bin'/p.name)
b=json.loads((old/'binaries.json').read_text())['after'];assert all(sha((C/'before-bin'/n).read_bytes())==h for n,h in b['binaries'].items());(C/'before-binding.json').write_text(json.dumps(b,indent=2)+'\n')
controls=[
 ('external_out_left','participant Cedar\nactor Finch\ncreate Finch\n[<- Finch: leave\nFinch -> Cedar: later\n',['outgoing left','actor ordinary successor']),
 ('external_out_right','participant Cedar\nparticipant Finch\nactivate Finch\ncreate Finch\nFinch ->]: leave\ndeactivate Finch\n',['outgoing right','active source']),
 ('external_in_left','participant Cedar\nactor Finch\ncreate Finch\n[-> Finch: arrive\nFinch -> Cedar: later\n',['incoming left','actor ordinary successor']),
 ('external_in_right','participant Cedar\nparticipant Finch\ncreate Cedar\ncreate Finch\nFinch <-]: arrive\ncreate Cedar\nFinch -> Cedar: ordinary create\n',['incoming right','pending overwrite then ordinary creation']),
 ('queue_bare','queue "<size:21>Mail</size>\\nsmall" as Mail <<dispatch-wide>>\nparticipant Cedar\nCedar -> Mail: enqueue\n',['bare stereotype','rich size first and tail']),
 ('queue_created','participant Cedar\ncreate queue "\\n<size:18>Mail</size>\\nend" as Mail <<work>>\nCedar -> Mail: enqueue\n',['created stereotype','blank first then rich stripe']),
 ('queue_box','box Hub\nqueue "short\\n<size:20>Mail</size>" as Mail <<dispatch-wide>>\nend box\nparticipant Cedar\nCedar -> Mail: enqueue\n',['boxed control','stereotype width and rich second']),
 ('queue_repeated','participant Cedar\nqueue "Work\\nroom" as Mail <<dispatch>>\ncreate Mail\nCedar -> Mail: first\ncreate Mail\nCedar -> Mail: second\n',['attached repeats','stereotype on created heads/tail'])]
p=C/'reference-inputs';p.mkdir();cases=[]
for name,body,axes in controls:
 for teoz in [False,True]:
  n=name+('_teoz' if teoz else '_legacy');s=p/(n+'.puml');s.write_text('@startuml\n'+('!pragma teoz true\n' if teoz else '')+body+'@enduml\n');cases.append(dict(name=n,source=str(s),source_sha256=sha(s.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+['teoz' if teoz else 'legacy'],intended_valid=True))
m=dict(schema_version=1,purpose='External CREATE consumer and complete queue Display bounded corrections',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw);bundle=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=s.name,sha256=sha(s.read_bytes()),base64=base64.b64encode(s.read_bytes()).decode()) for s in sorted(p.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z);print('manifest',sha(raw),'inputgzip',sha(z))
