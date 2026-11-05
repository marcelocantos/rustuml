# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,shutil
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest();old=R/'docs/parity-reviews/sequence-consumer-contract-packet/maker-results';review=old.parent/'checker-9784-astra-b4b6'
rows=json.loads((old/'all-inputs.json').read_text());fresh=json.loads((review/'fresh-pairs.json').read_text());assert len(rows)==2549 and len(fresh)==534
for r in fresh:r['cohort']='checker534'
(C/'all3083.json').write_text(json.dumps(rows+fresh,indent=2)+'\n');(C/'checker534.json').write_text(json.dumps(fresh,indent=2)+'\n');(C/'before-bin').mkdir()
for p in Path('/Users/marcelo/Library/Caches/rustuml-sequence-consumers-bce6/final-bin').iterdir():shutil.copy2(p,C/'before-bin'/p.name)
b=json.loads((old/'binaries.json').read_text())['after'];assert all(sha((C/'before-bin'/n).read_bytes())==h for n,h in b['binaries'].items());(C/'before-binding.json').write_text(json.dumps(b,indent=2)+'\n')
controls=[
 ('identity_keyword','participant "North Pier" as of\nparticipant South\nof -> South: seed\nnote over of #pink: keyed\n',True,['mandatory identity','keyword alias','color']),
 ('identity_prefix','participant "North Pier" as N\nparticipant South\nN -> South: seed\nrnote right of N : keyed\n',True,['optional prefix','inline rectangle']),
 ('identity_quoted','participant "North Pier"\n"North Pier" -> South: seed\nhnote over "North Pier"\nfirst\nsecond\nend hnote\n',True,['quoted identity','multiline hexagon']),
 ('identity_pair','participant "North Pier" as of\nof -> South: seed\nnote over of, South: paired\n',True,['keyword identity','pair']),
 ('vt_required','participant N\nparticipant S\nN -> S: seed\nactivate\vS\ndeactivate\vS\n',False,['Fox required space','only VT']),
 ('vt_global','participant "South Pier"\nN -> "South Pier": seed\nactivate\v"South Pier"\ndeactivate\v"South Pier"\n',True,['Fox global space bit','quoted name supplies space']),
 ('resume_vt','participant N\nautonumber\nN -> S: first\nautonumber stop\nN -> S: unnumbered\nautonumber\vresume\nN -> S: last\n',False,['Fox required space','resume']),
 ('resume_vt_format','participant N\nautonumber\nN -> S: first\nautonumber stop\nN -> S: unnumbered\nautonumber\vresume "Item 00"\nN -> S: last\n',True,['Fox global bit in format','resume']),
 ('box_nbsp','box\u00a0"Northern Shore" #pink\nparticipant N\nend box\nN -> S: cross\n',True,['entry NBSP','quoted box title']),
 ('across_ff','participant N\nN -> S: cross\nrnote\facross: field\n',True,['entry FF','across rectangle']),
 ('across_nbsp','participant N\nN -> S: cross\nhnote\u00a0across\nfield\nend hnote\n',True,['entry NBSP','multiline across hexagon']),
 ('accross_negative','participant N\nN -> S: cross\nnote accross: field\n',False,['recognized grammar','execution spelling rejection'])]
p=C/'reference-inputs';p.mkdir();cases=[]
for name,body,valid,axes in controls:
 for teoz in [False,True]:
  n=name+('_teoz' if teoz else '_legacy');s=p/(n+'.puml');s.write_text('@startuml\n'+('!pragma teoz true\n' if teoz else '')+body+'@enduml\n');cases.append(dict(name=n,source=str(s),source_sha256=sha(s.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',env={},invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+['teoz' if teoz else 'legacy'],intended_valid=valid))
m=dict(schema_version=1,purpose='Required note identity, Fox command whitespace bit and command entry grammar',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw);bundle=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=s.name,sha256=sha(s.read_bytes()),base64=base64.b64encode(s.read_bytes()).decode()) for s in sorted(p.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z);print(len(cases),'manifest',sha(raw),'gzip',sha(z))
