# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,shutil
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest();old=R/'docs/parity-reviews/sequence-filtered-display-packet/maker-results';review=old.parent/'checker-bce6bea-astra-f535'
rows=json.loads((old/'all-inputs.json').read_text());fresh=json.loads((review/'fresh-pairs.json').read_text());assert len(rows)==2371 and len(fresh)==148
(C/'all2519.json').write_text(json.dumps(rows+fresh,indent=2)+'\n');(C/'checker148.json').write_text(json.dumps(fresh,indent=2)+'\n');(C/'before-bin').mkdir()
for p in Path('/Users/marcelo/Library/Caches/rustuml-sequence-filtered-display-0cde/final-bin').iterdir():shutil.copy2(p,C/'before-bin'/p.name)
b=json.loads((old/'binaries.json').read_text())['after'];assert all(sha((C/'before-bin'/n).read_bytes())==h for n,h in b['binaries'].items());(C/'before-binding.json').write_text(json.dumps(b,indent=2)+'\n')
controls=[]
for kind in ['participant','actor','boundary','control','entity','database','collections','queue']:
 controls.append((kind+'_ordered',f'skinparam stereotypePosition bottom\n{kind} "Wide Birch\\n<size:18>Second</size>" as B <<role>>\nskinparam stereotypePosition top\n{kind} "Elm" as E <<broad role>>\nB -> E: notify\n',True,['kind '+kind,'declaration-time opposite orders','rich multiline label and unequal widths'],[False,True]))
for name,gap,valid in [('nbsp','\u00a0',True),('em','\u2003',False),('narrow','\u202f',False)]:
 controls.append(('message_gap_'+name,f'participant Birch\nparticipant Elm\nBirch -> Elm: initial\nBirch{gap}->{gap}Elm: next\n',valid,['Pattern2 message whitespace',name],[False]))
 controls.append(('terminator_gap_'+name,f'participant Birch\nhnote over Birch\nfirst\nsecond\nend{gap}hnote\nBirch -> Birch: later\n',valid,['Pattern2 terminator whitespace',name],[True]))
for side,shape in [('left','rnote'),('right','hnote')]:controls.append(('self_'+side+'_'+shape,f'participant Birch\nBirch -> Birch: self\n{shape} {side}\n<size:17>first</size>\nsecond\nthird\nend note\nBirch -> Elm: next\n',True,['Teoz self component selection',side,shape,'rich multiline'],[False,True]))
for name,note in [('inline','note over: forbidden\n'),('multiline','note over\nforbidden\nend note\n')]:controls.append(('bare_over_'+name,'participant Birch\n[-> Birch: arrive\n'+note,False,['bare over requires named target',name,'external predecessor'],[False,True]))
p=C/'reference-inputs';p.mkdir();cases=[]
for name,body,valid,axes,engines in controls:
 for teoz in engines:
  n=name+('_teoz' if teoz else '_legacy');s=p/(n+'.puml');s.write_text('@startuml\n'+('!pragma teoz true\n' if teoz else '')+body+'@enduml\n');cases.append(dict(name=n,source=str(s),source_sha256=sha(s.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',env={},invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+['teoz' if teoz else 'legacy'],intended_valid=valid))
m=dict(schema_version=1,purpose='Pattern2 whitespace/rejection contracts, all-kind ordered Display and Teoz self-note components',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw);bundle=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=s.name,sha256=sha(s.read_bytes()),base64=base64.b64encode(s.read_bytes()).decode()) for s in sorted(p.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z);print(len(cases),'manifest',sha(raw),'gzip',sha(z))
