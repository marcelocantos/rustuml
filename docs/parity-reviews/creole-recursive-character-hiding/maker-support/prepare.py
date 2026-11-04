# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,base64,hashlib,shutil
from pathlib import Path
S=Path(__file__).parent;N=S/'new';N.mkdir(exist_ok=True);sha=lambda b:hashlib.sha256(b).hexdigest();old=Path('/Users/marcelo/Library/Caches/rustuml-style-wbs-maker');checker=Path('/Users/marcelo/Library/Caches/rustuml-style-wbs-checker-20261105')
cohorts=['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2']
for name in ['replay.py','run_replay.py','probe','probe.rs'] :shutil.copy2(old/name,S/name)
for cohort,origin in [(c,old) for c in cohorts]+[('checker30',checker),('supplement8',checker)]:
 data=(origin/(cohort+'-membership.json')).read_bytes();(S/(cohort+'-original-membership.json')).write_bytes(data);rows=json.loads(data)
 for row in rows:
  d=S/'frozen'/row['id'];d.mkdir(parents=True,exist_ok=True)
  for field in ['source_capture','golden_capture','reference_source_capture']:
   if field not in row:continue
   p=Path(row[field]);b=p.read_bytes();dest=d/('input.puml' if field=='source_capture' else 'java.svg' if field=='golden_capture' else 'java.source');dest.write_bytes(b);row[field]=str(dest)
   if field!='reference_source_capture':assert sha(b)==row[field.replace('_capture','_sha256')]
 (S/(cohort+'-membership.json')).write_text(json.dumps(rows,indent=2)+'\n')
# New input bytes are never rewritten after this preparation.
seq=lambda lines:'@startuml\nCypress -> Linden : '+r'\n'.join(lines)+'\n@enduml\n'
wbs=lambda lines:'@startwbs\n'+'\n'.join(lines)+'\n@endwbs\n'
cases={
'html_escape_stream':seq([r'P<b>ivy\~</b>Q',r'P<i>~<b>ash</b></i>Q',r'P<u>a\~</u>Q',r'P<s>a\~</s>Q',r'P<w>a\~</w>Q']),
'overlapping_openers':seq(['P~'+d[0]+d+'elm'+d+'Q' for d in ['**','//','__','--']]),
'wave_tilde_pairs':seq([r'P~~a\~b~~Q',r'P~~a\~\~b~~Q',r'P<b>~~a~~</b>Q',r'P~a~~b~~Q']),
'recursive_hiding':seq([r'P<b><i>a\~</i>~***elm**</b>Q',r'P<size:13><color:blue>a\~</color></size>Q',r'P<b>~</b>elm</b>Q']),
'literal_neighbors':seq([r'P~#~.~[~]~_~-~"~*~/~<Q',r'P~a~ΩQ',r'P\~***elm**Q',r'P https://example.test/path Q']),
'empty_parameter_runs':seq([r'P<size:13></size>Q<color:black></color>R',r'P<size:13>x</size>Q<color:black>y</color>R',r'P<b><size:13></size>Q<color:black></color>R</b>S']),
'consumed_parameter_runs':seq([r'P<size:19><$twig></size>Q',r'P<color:red><$twig></color>Q',r'<size:19></size>',r'<color:red></color>']),
'wbs_line_commands':wbs([r'* Pine\nP<size:12></size>Q',r'** P<color:black></color>Q\n\nR',r'***_ P<size:12><$twig></size>Q',r'--_ P<color:black><$twig></color>Q']),
'wbs_hidden_leaf':wbs(['* Pine',r'**_ <b>a\~</b>',r'***_ P~***elm**Q',r'--_ <i>a\~</i>']),
'note_commands':'@startuml\nparticipant Cypress\nnote right of Cypress\nP<size:13></size>Q\nP<b>a\\~</b>Q\nend note\n@enduml\n',
'activity_commands':'@startuml\nstart\n:P<size:13></size>Q;\n:P~***elm**Q;\nstop\n@enduml\n',
'wbs_escape_lines':wbs([r'* Pine\nP<b>a\~</b>Q',r'**_ P~***elm**Q\n\nR',r'-- P\\nQ\nR'])}
rows=[]
for name,source in cases.items():
 p=N/(name+'.puml');p.write_text(source);rows.append(dict(source=str(p),mechanism='creole-recursive-character-hiding' if any(x in name for x in ['escape','overlap','hiding','hidden','literal','wave']) else 'creole-parameter-command-flush',axes=['escaping or command body cardinality','recursive style and family consumer'],cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={},negative=False,intended_validity='intended_valid'))
(S/'manifest.json').write_text(json.dumps(rows,indent=2)+'\n');res=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.glob('*.puml'))];q=json.loads((old/'request.json').read_bytes());q.update(manifest_path=str(S/'manifest.json'),manifest_sha256=sha((S/'manifest.json').read_bytes()),case_count=len(rows),resources=res,case_contracts=[dict(source=x['source'],source_sha256=sha(Path(x['source']).read_bytes()),cwd='<worker>/new',timezone='GMT+11:00',env_overrides={},svg_argv=['reference','--cwd','<worker>/new','--timezone','GMT+11:00','--','-pipe','-tsvg']) for x in rows]);(S/'request.json').write_text(json.dumps(q,indent=2)+'\n');files=[dict(local_path=x['local_path'],remote_relative_path=x['remote_relative_path'],sha256=x['sha256'],base64=base64.b64encode(Path(x['local_path']).read_bytes()).decode()) for x in res];(S/'input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(dict(request=q,manifest=rows,files=files)).encode(),mtime=0));print({p.name:sha(p.read_bytes()) for p in [S/'request.json',S/'manifest.json',S/'input-bundle.json.gz']})
