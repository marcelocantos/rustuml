# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,base64,hashlib,shutil
from pathlib import Path
S=Path(__file__).parent;N=S/'new';N.mkdir(exist_ok=True);sha=lambda b:hashlib.sha256(b).hexdigest();old=Path('/Users/marcelo/Library/Caches/rustuml-creole-hide-flush-maker');checker=Path('/Users/marcelo/Library/Caches/rustuml-creole-hide-checker-20261105')
cohorts=['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12']
for name in ['replay.py','run_replay.py','probe','probe.rs']:shutil.copy2(old/name,S/name)
for cohort,origin in [(c,old) for c in cohorts]+[('checker28',checker)]:
 raw=(origin/(cohort+'-membership.json')).read_bytes();(S/(cohort+'-original-membership.json')).write_bytes(raw);rows=json.loads(raw)
 for row in rows:
  d=S/'frozen'/row['id'];d.mkdir(parents=True,exist_ok=True)
  for field in ['source_capture','golden_capture','reference_source_capture']:
   if field not in row:continue
   b=Path(row[field]).read_bytes();p=d/('input.puml' if field=='source_capture' else 'java.svg' if field=='golden_capture' else 'java.source');p.write_bytes(b);row[field]=str(p)
   if field!='reference_source_capture':assert sha(b)==row[field.replace('_capture','_sha256')]
 (S/(cohort+'-membership.json')).write_text(json.dumps(rows,indent=2)+'\n')
seq=lambda xs:'@startuml\nJuniper -> Poplar : '+r'\n'.join(xs)+'\n@enduml\n'
note=lambda xs:'@startuml\nparticipant Juniper\nnote right of Juniper\n'+'\n'.join(xs)+'\nend note\n@enduml\n'
wbs=lambda xs:'@startwbs\n'+'\n'.join(xs)+'\n@endwbs\n'
cases={
'size_rejected':note([r'L<size:~13>oak</size>R',r'L<size:1\~3>oak</size>R',r'L<size:13~>oak</size>R']),
'color_rejected':note([r'L<color:~#123456>oak</color>R',r'L<color:bl\~ue>oak</color>R',r'L<color:red!>oak</color>R']),
'size_forms':seq(['L<SiZe :13 >oak</SIZE>R','L<size\t13>oak</SiZe>R','L<size:13></SIZE>R']),
'color_forms':seq(['L<CoLoR :navy >oak</COLOR>R','L<color:#123>oak</CoLoR>R','L<color:blue></color>R']),
'font_priority':seq(['L<font size="13" color="navy">oak</font>R','L<font color=red size=13 color=blue>oak</FONT>R','L<font size=13></font>R']),
'font_rejected':seq(['L<font>oak</font>R','L<fontish:red>oak</font>R','L<font:>oak</font>R']),
'font_fallback':seq(["L<font color='red'>oak</font>R",r'L<font size=~13>oak</font>R',r'L<font color=~#123456>oak</font>R']),
'font_family':wbs(['* L<font:SansSerif>oak</font>R','** L<font Dialog>bud</FONT>R','--_ L<font:Monospaced>bud</font>R']),
'back_rejected':seq([r'L<back:~#123456>oak</back>R',r'L<back:re\~d>oak</back>R',r'L<back:#12>oak</back>R']),
'back_forms':seq(['L<BaCk:red>oak</bAcK>R','L<back:#123456>oak</back>R','L<back:blue><$twig></back>R']),
'image_rejected':seq(['L<img:>oakR','L<img:absent.png{scale=no}>oakR','L<img:absent.png{width=2}>oak<$twig>R']),
'icon_rejected':seq(['L<&>oakR','L<&home{scale=no}>oakR',r'L<&home{color=~#123456}>oakR']),
'sprite_neighbor':seq([r'L<$twig{color=~#123456}>oakR','L<$twig>oakR',r'L<$twig{scale=bad}>oakR']),
'wbs_parameters':wbs([r'* L<size:~12>oak</size>R\nEnd',r'**_ L<color:~#123456>bud</color>R',r'*** L<size:12></size>R',r'--_ L<font size=12 color=blue>bud</font>R']),
'inner_candidate':seq([r'L<size:~13<color:red>oak</color>R',r'L<color:~#<$twig>>oakR',r'L<img:{bad}<b>oak</b>R']),
'class_member': '@startuml\nclass Juniper {\n L<size:~13>oak</size>R\n L<color:~#123456>bud</color>R\n}\n@enduml\n'}
rows=[]
for name,source in cases.items():
 p=N/(name+'.puml');p.write_text(source);rows.append(dict(source=str(p),mechanism='creole-parameter-candidate-grammar',axes=[name,'hidden parameter grammar and family consumer'],cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={},negative=False,intended_validity='intended_valid'))
(S/'manifest.json').write_text(json.dumps(rows,indent=2)+'\n');resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.glob('*.puml'))];request=json.loads((old/'request.json').read_bytes());request.update(manifest_path=str(S/'manifest.json'),manifest_sha256=sha((S/'manifest.json').read_bytes()),case_count=len(rows),resources=resources,case_contracts=[dict(source=x['source'],source_sha256=sha(Path(x['source']).read_bytes()),cwd='<worker>/new',timezone='GMT+11:00',env_overrides={},svg_argv=['reference','--cwd','<worker>/new','--timezone','GMT+11:00','--','-pipe','-tsvg']) for x in rows]);(S/'request.json').write_text(json.dumps(request,indent=2)+'\n');files=[dict(local_path=x['local_path'],remote_relative_path=x['remote_relative_path'],sha256=x['sha256'],base64=base64.b64encode(Path(x['local_path']).read_bytes()).decode()) for x in resources];(S/'input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(dict(request=request,manifest=rows,files=files)).encode(),mtime=0));print({p.name:sha(p.read_bytes()) for p in [S/'request.json',S/'manifest.json',S/'input-bundle.json.gz']})
