# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,base64,hashlib,shutil
from pathlib import Path
S=Path(__file__).parent;N=S/'new';N.mkdir(exist_ok=True);(S/'logs').mkdir(exist_ok=True);sha=lambda b:hashlib.sha256(b).hexdigest();old=S.parent/'rustuml-creole-parameter-maker';checker=S.parent/'rustuml-creole-parameter-checker-e6a64f42'
cohorts=['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12','checker28','fresh16','golden-regressions']
for name in ['replay.py','run_replay.py','probe','probe.rs']:shutil.copy2(old/name,S/name)
for cohort,origin in [(c,old) for c in cohorts]+[('checker36',checker)]:
 raw=(origin/(cohort+'-membership.json')).read_bytes();(S/(cohort+'-original-membership.json')).write_bytes(raw);rows=json.loads(raw)
 for row in rows:
  d=S/'frozen'/row['id'];d.mkdir(parents=True,exist_ok=True)
  for field in ['source_capture','golden_capture','reference_source_capture']:
   if field not in row:continue
   b=Path(row[field]).read_bytes();p=d/('input.puml' if field=='source_capture' else 'java.svg' if field=='golden_capture' else 'java.source');p.write_bytes(b);row[field]=str(p)
   if field!='reference_source_capture':assert sha(b)==row[field.replace('_capture','_sha256')]
 (S/(cohort+'-membership.json')).write_text(json.dumps(rows,indent=2)+'\n')
seq=lambda xs:'@startuml\nElm -> Cedar : '+r'\n'.join(xs)+'\n@enduml\n'
note=lambda xs:'@startuml\nparticipant Elm\nnote right of Elm\n'+'\n'.join(xs)+'\nend note\n@enduml\n'
act=lambda xs:'@startuml\nstart\n:'+r'\n'.join(xs)+';\nstop\n@enduml\n'
cases={
'color_invalid_runs':seq(['q<color:#HHHHHH>elm</color>r','q<color:NoSuchTone>elm</color>r','q<color:#12>elm</color>r']),
'color_nested_current':seq(['q<color:navy>a<color:#GGHHII>elm</color>b</color>r','q<color:red><color:UnknownTone></color>r</color>s']),
'color_eol_activity':act(['q<color:NoSuchTone>elm','q<color:blue>a<color:#12>elm']),
'color_valid_neighbors':note(['q<color:red>elm</color>r','q<color:#112233>elm</color>r','q<color:#123>elm</color>r']),
'color_registry_boundary':seq(['q<color:BUSINESS>elm</color>r','q<color:RebeccaPurple>elm</color>r','q<color:#red>elm</color>r']),
'back_invalid':seq(['q<back:NoSuchTone>elm</back>r','q<back:RebeccaPurple>elm</back>r','q<back:#GGGGGG>elm</back>r']),
'back_nested':act(['q<back:blue>a<back:NoSuchTone>elm</back>b</back>r','q<color:navy><back:NoSuchTone>elm</back>r</color>']),
'back_invalid_gradient':seq(['q<back:red-NoSuchTone>elm</back>r','q<back:NoSuchTone/blue>elm</back>r','q<back:red-blue>elm</back>r']),
'font_invalid_color':seq(['q<font color=NoSuchTone>elm</font>r']),
'font_invalid_with_size':act(['q<font size=15 color=NoSuchTone>elm</font>r']),
'image_markup_faces':note(['q<b><img:fallback-missing572.png></b>r','q<i><img:fallback-missing572.png></i>r','q<s><u><img:fallback-missing572.png></u></s>r']),
'image_markup_config':note(['q<color:red><back:yellow><img:fallback-missing572.png></back></color>r','q<font:Serif><size:21><img:fallback-missing572.png></size></font>r','q<sub><img:fallback-missing572.png></sub>r']),
'image_plain_scale':act(['q<img:fallback-missing572.png>r','q<img:fallback-missing572.png{scale=2}>r']),
'image_base_bold':'@startwbs\n* <img:fallback-missing572.png>\n** q<b><img:fallback-missing572.png></b>r\n@endwbs\n',
'color_wbs_member':'@startwbs\n* Oak\n** q<color:navy>a<color:NoSuchTone>elm</color>b</color>r\n**_ q<back:NoSuchTone>elm</back>r\n@endwbs\n',
'grammar_neighbors':seq([r'q<color:~#123456>elm</color>r','q<back:>elm</back>r','q<color:NoSuchTone><$twig>elm</color>r'])}
rows=[]
for name,source in cases.items():
 p=N/(name+'.puml');p.write_text(source);negative=name.startswith('font_invalid');rows.append(dict(source=str(p),mechanism='creole-image-fallback-style' if name.startswith('image') else 'creole-color-resolution-fallback',axes=[name,'resolution versus candidate selection; nested versus base style'],cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={},negative=negative,intended_validity='intended_negative' if negative else 'intended_valid'))
(S/'manifest.json').write_text(json.dumps(rows,indent=2)+'\n');resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.glob('*.puml'))];request=json.loads((old/'request.json').read_bytes());request.update(manifest_path=str(S/'manifest.json'),manifest_sha256=sha((S/'manifest.json').read_bytes()),case_count=len(rows),resources=resources,case_contracts=[dict(source=x['source'],source_sha256=sha(Path(x['source']).read_bytes()),cwd='<worker>/new',timezone='GMT+11:00',env_overrides={},svg_argv=['reference','--cwd','<worker>/new','--timezone','GMT+11:00','--','-pipe','-tsvg']) for x in rows]);(S/'request.json').write_text(json.dumps(request,indent=2)+'\n');files=[dict(local_path=x['local_path'],remote_relative_path=x['remote_relative_path'],sha256=x['sha256'],base64=base64.b64encode(Path(x['local_path']).read_bytes()).decode()) for x in resources];(S/'input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(dict(request=request,manifest=rows,files=files)).encode(),mtime=0));print({p.name:sha(p.read_bytes()) for p in [S/'request.json',S/'manifest.json',S/'input-bundle.json.gz']})
