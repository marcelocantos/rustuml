# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,base64,hashlib
from pathlib import Path
S=Path(__file__).parent; N=S/'new'; N.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
seq=lambda ls:'@startuml\nJuniper -> Alder : '+r'\n'.join(ls)+'\n@enduml\n'
activity=lambda ls:'@startuml\nstart\n'+''.join(':'+x+';\n' for x in ls)+'stop\n@enduml\n'
state=lambda ls:'@startuml\nstate Grove\n'+''.join('Grove : '+x+'\n' for x in ls)+'@enduml\n'
note=lambda ls:'@startuml\nparticipant Grove\nnote right of Grove\n'+'\n'.join(ls)+'\nend note\n@enduml\n'
member=lambda ls:'@startuml\nclass Grove {\n'+ '\n'.join(ls)+'\n}\n@enduml\n'
wbs=lambda ls:'@startwbs\n'+'\n'.join(ls)+'\n@endwbs\n'
cases=[]
def add(name,source,mechanism,axes):cases.append((name,source,mechanism,axes))
hide='creole-recursive-character-hiding';flush='creole-parameter-command-flush'
for name,wrap in [('activity',activity),('state',state),('note',note),('member',member)]:
 add('hide_'+name+'_edges',wrap([r'~***fir**Z',r'<b>~***fir**Z\~</b>~',r'X<i>\~<u>~_fir</u>~</i>Y']),hide,['start/end escape position','nested style depth','consumer '+name])
for name,d in [('bold','**'),('italic','//'),('underline','__'),('strike','--'),('wave','~~')]:
 add('overlap_'+name,note(['X~'+d[0]+d+'cedar'+d+'Y','X<b>~'+d[0]+d+'birch'+d+'</b>Y']),hide,['each delimiter family','plain versus nested command','note consumer'])
add('hide_tilde_depth',seq([r'X~~fir~~Y',r'X~~~fir~~Y',r'X<b>~~~fir~~</b>Y',r'X<i><b>\~~~fir~~</b></i>Y',r'X~~\~\~fir\~\~~~Y']),hide,['double/triple tilde','recursion depth','escaped wave closers'])
add('hide_closing_capture',note([r'X<b>a~</b>c</b>Y',r'X<size:13>a~</size>b</size>Y',r'X<color:blue>a~</color>b</color>Y',r'X<b><i>~</i>x</i></b>Y']),hide,['escaped closing tags','style versus parameter capture'])
add('hide_parameter_size',note([r'X<size:~13>fir</size>Y',r'X<size:1\~3>fir</size>Y',r'X<size:13~>fir</size>Y']),hide,['escape in numeric parameter','valid literal fallback'])
add('hide_parameter_color',note([r'X<color:~#123456>fir</color>Y',r'X<color:bl\~ue>fir</color>Y',r'X<color:~red>fir</color>Y']),hide,['hidden hash in parameter','unrecognized color fallback'])
add('hide_link_target',note([r'X[[https://example.test/~#part fir]]Y',r'X[[https://example.test/a\~b fir]]Y',r'X[[https://example.test/x{tip~[x~]} fir]]Y']),hide,['hidden characters in URL versus tooltip','attribute restoration boundary'])
add('hide_link_label',note([r'X[[https://example.test ~***fir**]]Y',r'X[[https://example.test <b>a\~</b>]]Y',r'X~[[https://example.test fir]]Y']),hide,['label recursion versus raw label','escaped bracket opener'])
add('hide_raw_regions',note([r'X""a\~ ~***fir**""Y',r'X<code>a\~ ~***fir**</code>Y',r'X<plain>a\~ ~***fir**</plain>Y']),hide,['monospace and plain command regions','recursive escape boundary'])
add('hide_unicode_sprite',activity([r'X\~<$absent>~***fir**Y',r'X~<$absent>Y',r'Xé\~<b>~***fir**</b>Y',r'X~𐐀<b>\~</b>Y']),hide,['sprite adjacency','BMP versus supplementary Unicode'])
add('hide_wbs_composed',wbs([r'*_ ~***fir**Z\n<b>\~</b>',r'**_ <i>~***elm**</i>\n~',r'***_ X<b>a~</b>c</b>Y',r'--_ X~~~fir~~Y']),hide,['multiline shape-free labels','composed versus leaf depth and side'])
for name,wrap in [('activity',activity),('state',state),('note',note),('member',member)]:
 add('flush_'+name+'_positions',wrap(['<size:13></size>X<color:black></color>','A<size:13></size><color:black></color>B','<color:blue><size:13></size></color>X<color:red>Q</color>']),flush,['leading/trailing empty commands','adjacent/nested empty followed by sibling','consumer '+name])
add('flush_eol_nested',note(['X<size:13>Y<size:13>','X<color:black>Y<color:black>','X<size:13><color:black><$absent>','<size:13><color:black><$absent>']),flush,['recursive EOL capture','consumed-empty versus actual stripe fallback'])
add('flush_wbs_leaf',wbs(['* Grove',r'**_ <size:12></size>X\nY<color:black></color>',r'--_ <color:blue></color><size:12></size>\n<color:black><$absent></color>']),flush,['multiline empty leading/trailing stripes','leaf side and atomless line'])
add('flush_wbs_composed',wbs([r'*_ <size:12></size>X\nY<color:black></color>',r'**_ A<size:12></size><color:black></color>B',r'***_ <color:blue><size:12></size></color>X<color:red>Q</color>',r'--_ <size:12></size>']),flush,['composed versus leaf','nested command and adjacent sibling','blank stripe'])
add('flush_equal_style',seq(['X<size:13>Y</size>Z','X<color:black>Y</color>Z','X<b>A<size:13></size>B</b>Y','X<color:black>A<color:black></color>B</color>Y']),flush,['equal style versus restored style','empty nested inside nonempty command'])
# Source-guided controls isolate the maker residuals without changing old inputs.
add('residual_ascii_neighbor',seq([r'P~#~.~[~]~_~-~"~*~/~<Q',r'P~a~WQ',r'P\~***elm**Q',r'P https://example.test/path Q']),hide,['Omega replaced by ASCII only','existing maker residual isolation'])
add('residual_omega_only',seq([r'P~a~ΩQ']),hide,['single Unicode-width row','no command recursion'])
rows=[]
for name,source,mechanism,axes in cases:
 p=N/(name+'.puml');p.write_text(source)
 rows.append(dict(source=str(p),mechanism=mechanism,axes=axes,cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={},negative=False,intended_validity='intended_valid'))
manifest=json.dumps(rows,indent=2)+'\n';(S/'manifest.json').write_text(manifest)
resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.glob('*.puml'))]
q=json.loads((S/'maker/request.json').read_bytes());q.update(manifest_path=str(S/'manifest.json'),manifest_sha256=sha(manifest.encode()),case_count=len(rows),resources=resources,case_contracts=[dict(source=r['source'],source_sha256=sha(Path(r['source']).read_bytes()),cwd='<worker>/new',timezone='GMT+11:00',env_overrides={},svg_argv=['reference','--cwd','<worker>/new','--timezone','GMT+11:00','--','-pipe','-tsvg']) for r in rows]);(S/'request.json').write_text(json.dumps(q,indent=2)+'\n')
files=[dict(local_path=r['local_path'],remote_relative_path=r['remote_relative_path'],sha256=r['sha256'],base64=base64.b64encode(Path(r['local_path']).read_bytes()).decode()) for r in resources]
(S/'input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(dict(request=q,manifest=rows,files=files)).encode(),mtime=0))
print('Cases',len(rows));print(json.dumps({p.name:sha(p.read_bytes()) for p in [S/'request.json',S/'manifest.json',S/'input-bundle.json.gz']},indent=2))
for p in [*N.glob('*.puml'),S/'request.json',S/'manifest.json',S/'input-bundle.json.gz']:p.chmod(0o444)
