# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,shutil
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest()
old=R/'docs/parity-reviews/sequence-external-display-packet/maker-results';review=old.parent/'checker-0cde5d7-astra'
rows=json.loads((old/'all-inputs.json').read_text());fresh=json.loads((review/'fresh-pairs.json').read_text())
assert len(rows)==2281 and len(fresh)==64
(C/'all2345.json').write_text(json.dumps(rows+fresh,indent=2)+'\n');(C/'checker64.json').write_text(json.dumps(fresh,indent=2)+'\n')
(C/'before-bin').mkdir(exist_ok=True)
for p in Path('/Users/marcelo/Library/Caches/rustuml-sequence-external-display-ae1/final-bin').iterdir():shutil.copy2(p,C/'before-bin'/p.name)
b=json.loads((old/'binaries.json').read_text())['after'];assert all(sha((C/'before-bin'/n).read_bytes())==h for n,h in b['binaries'].items());(C/'before-binding.json').write_text(json.dumps(b,indent=2)+'\n')
controls=[
('hide_kinds','hide stereotype\nqueue "LongMailbox" as M <<x>>\nactor Gate <<role>>\nparticipant Hub <<service>>\nHub -> M: put\nM -> Gate: send\n',['global hide','three kinds']),
('hide_created','participant Hub\ncreate queue "LongMailbox" as M <<x>>\nHub -> M: open\nhide stereotypes\n',['global final state','created queue']),
('show_restore','hide stereotype\nqueue Mail <<work>>\nparticipant Hub\nshow stereotype\nHub -> Mail: put\n',['show restoration','head and tail']),
('bottom_order','skinparam stereotypePosition bottom\nqueue "<size:20>Mailbox</size>\\nsmall" as M <<work>>\nskinparam stereotypePosition top\nqueue Other <<top>>\nM -> Other: put\n',['declaration order capture','multiline rich label']),
('bottom_created','skinparam stereotypePosition bottom\nparticipant Hub\ncreate queue Mail <<role>>\nHub -> Mail: open\n',['bottom display','created component']),
('rect_rows','participant Hub\nrnote right of Hub\none\ntwo\nthree\nfour\nend rnote\nHub -> Hub: later\n',['rectangular','four fractional rows']),
('hex_rows','participant Hub\nhnote over Hub\n<size:17>one</size>\ntwo\nthree\nfour\nfive\nend hnote\nHub -> Hub: later\n',['hexagonal','five mixed size rows']),
('fold_rows','participant Hub\nnote right of Hub\none\ntwo\nthree\nfour\nend note\nHub -> Hub: later\n',['folded control','four rows']),
('exo_note_in','participant Hub\n[-> Hub: arrive\nnote right: attached\n',['external incoming','bare inline note']),
('exo_note_out','participant Hub\nHub ->]: leave\nnote left\nfirst\nsecond\nend note\n',['external outgoing','multiline note']),
('exo_stale_owner','participant Birch\nparticipant Hub\nBirch -> Hub: prior\n[<- Birch: leave\nnote right: current owner\n',['normalized reverse','ordinary predecessor']),
('exo_right_in','participant Hub\nHub <-]: arrive\nnote left: current owner\n',['right border incoming','bare left note'])]
p=C/'reference-inputs';p.mkdir(exist_ok=True);cases=[]
for name,body,axes in controls:
 for teoz in [False,True]:
  n=name+('_teoz' if teoz else '_legacy');s=p/(n+'.puml');s.write_text('@startuml\n'+('!pragma teoz true\n' if teoz else '')+body+'@enduml\n');cases.append(dict(name=n,source=str(s),source_sha256=sha(s.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',env={},invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+['teoz' if teoz else 'legacy'],intended_valid=True))
m=dict(schema_version=1,purpose='Filtered ordered participant Display, aggregate note paint height and external note ownership',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw);bundle=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=s.name,sha256=sha(s.read_bytes()),base64=base64.b64encode(s.read_bytes()).decode()) for s in sorted(p.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z);print('manifest',sha(raw),'inputgzip',sha(z))
