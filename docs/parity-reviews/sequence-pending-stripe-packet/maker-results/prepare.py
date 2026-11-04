# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import json,hashlib,gzip,base64,shutil,subprocess
C=Path(__file__).parent;R=Path.cwd();h=lambda b:hashlib.sha256(b).hexdigest();prior=R/'docs/parity-reviews/sequence-component-display-packet/maker-results';review=prior.parent/'checker-711-astra'
old=json.loads((prior/'all-inputs.json').read_text());fresh=json.loads((review/'fresh64.json').read_text());fresh=[dict(source=r['source'],golden=r['golden'],cohort='checker64',axes=r['axes']) for r in fresh];assert len(old)==2083 and len(fresh)==64
(C/'all2147.json').write_text(json.dumps(old+fresh,indent=2)+'\n');(C/'checker64.json').write_text(json.dumps(fresh,indent=2)+'\n')
(C/'before-bin').mkdir(exist_ok=True)
for p in Path('/Users/marcelo/Library/Caches/rustuml-sequence-component-672/final-bin').iterdir():shutil.copy2(p,C/'before-bin'/p.name)
shutil.copy2(prior/'final-binaries.json',C/'before-binding.json')
for p,v in json.loads((C/'before-binding.json').read_text())['source_files'].items():assert h(subprocess.check_output(['git','show','711dbca7:'+p]))==v
handoff=C/'handoff';handoff.mkdir(exist_ok=True);S=Path('/Users/marcelo/Library/Caches/rustuml-style-wbs-maker')
items=[]
for name,src,ref in [('present_bold_empty',S/'frozen/fresh26/present_bold_empty/present_bold_empty.puml',S/'frozen/fresh26/present_bold_empty/java.svg.stdout'),('style_parameter_contrast',S/'new/style_parameter_contrast.puml',S/'fresh-reference/style_parameter_contrast/java.svg.stdout')]:
 for p,q in [(src,handoff/(name+'.puml')),(ref,handoff/(name+'.svg'))]:shutil.copy2(p,q)
 items.append(dict(source=str(handoff/(name+'.puml')),golden=str(handoff/(name+'.svg')),cohort='shared-handoff2',original_source=str(src),original_reference=str(ref)))
(C/'handoff2.json').write_text(json.dumps(items,indent=2)+'\n')
controls=[
('pending_same','participant Cedar\nactor Roe\ncreate Roe\ncreate Roe\nCedar -> Roe: one\nCedar -> Roe: ordinary\ncreate Roe\nCedar -> Roe: two\n',['overwritten same participant','attached repeated creation']),
('pending_different','participant Cedar\nactor Roe\nparticipant Elm\ncreate Roe\ncreate Elm\nCedar -> Elm: one\nCedar -> Roe: ordinary\ncreate Roe\nCedar -> Roe: two\n',['overwritten different participant','actor and rectangle']),
('pending_incompatible','participant Cedar\nparticipant Elm\ncreate Elm\nElm -> Cedar: wrong receiver\n',['incompatible receiver','error status']),
('pending_self','participant Cedar\ncreate Cedar\nCedar -> Cedar: incompatible self\n',['self incompatibility','error status']),
('queue_first','queue "<size:23>First</size>\\nEnd" as Mail\nparticipant Gate\nGate -> Mail: enqueue\n',['rich first stripe','initial and tail']),
('queue_created','participant Gate\ncreate queue "<size:9>Small</size>\\n<size:21>Second</size>" as Mail\nGate -> Mail: enqueue\n',['small first / large second','created queue']),
('message_rise','participant Cedar\nparticipant Elm\nCedar -> Elm: small\\n<size:23>Tall</size>\\nend\n',['rising and falling ascent','ordinary message']),
('message_fall','participant Cedar\nCedar -> Cedar: <size:25>First</size>\\nsecond\\n<size:9>Last</size>\n',['large first small final','self message']),
('self_halves','participant Cedar\nCedar /- Cedar: reverse top\nCedar -\\ Cedar: forward top\nCedar \\- Cedar: reverse bottom\nCedar -/ Cedar: forward bottom\n',['top and bottom parts','forward/reverse order']),
('self_live','participant Cedar\nactivate Cedar\nCedar o/-o Cedar: live circle\nactivate Cedar\nCedar /- Cedar: nested\ndeactivate Cedar\nCedar \\- Cedar: lower\ndeactivate Cedar\n',['active depth','decorated reversed half'])]
inputs=C/'reference-inputs';inputs.mkdir(exist_ok=True);cases=[]
for name,body,axes in controls:
 for teoz in [False,True]:
  n=name+('_teoz' if teoz else '_legacy');p=inputs/(n+'.puml');assert not p.exists();b=('@startuml\n'+('!pragma teoz true\n' if teoz else '')+body+'@enduml\n').encode();p.write_bytes(b);cases.append(dict(name=n,source=str(p),source_sha256=h(b),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+['teoz' if teoz else 'legacy'],intended_valid=name not in ['pending_incompatible','pending_self']))
m=dict(schema_version=1,purpose='Bounded sequence pending-create / stripe-origin / half-self corrections; source-derived controls',cases=cases);b=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(b)
bundle=dict(manifest=m,manifest_sha256=h(b),files=[dict(path=p.name,sha256=h(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(inputs.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z);print(dict(manifest_sha256=h(b),input_gzip_sha256=h(z)))
