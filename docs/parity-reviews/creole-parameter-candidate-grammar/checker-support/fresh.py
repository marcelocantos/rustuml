# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,hashlib,base64
from pathlib import Path
S=Path(__file__).parent;N=S/'new';N.mkdir(exist_ok=True);sha=lambda b:hashlib.sha256(b).hexdigest()
def seq(xs):return '@startuml\nAlder -> Birch : '+r'\n'.join(xs)+'\n@enduml\n'
def note(xs):return '@startuml\nparticipant Alder\nnote right of Alder\n'+'\n'.join(xs)+'\nend note\n@enduml\n'
def act(xs):return '@startuml\nstart\n:' + r'\n'.join(xs)+';\nstop\n@enduml\n'
def state(xs):return '@startuml\nstate Grove\n'+'\n'.join('Grove : '+x for x in xs)+'\n@enduml\n'
def cls(xs):return '@startuml\nclass Grove {\n'+'\n'.join(xs)+'\n}\n@enduml\n'
def wbs(xs):return '@startwbs\n* Grove\n'+'\n'.join('** '+x for x in xs)+'\n@endwbs\n'
cases=[]
def add(name,source,axes,alias=False,negative=False):cases.append(dict(name=name,source_text=source,mechanism='creole-parameter-candidate-grammar-alias-compatibility' if alias else 'creole-parameter-candidate-grammar',axes=axes,negative=negative,intended_validity='intended_negative' if negative else 'intended_valid'))
add('size_zero',act(['a<size:0>seed</size>z']),['zero numeric conversion','activity consumer'])
add('size_signed_decimal',state(['a<size:+14>b</size>z','a<size:-14>b</size>z','a<size:14.0>b</size>z']),['signed and decimal rejection','state descriptions'])
add('size_space_boundaries',note(['a<size:\t014\u00a0>b</SIZE>z','a<size\u00a014>b</size>z','a<size: 14 >b</size>z']),['ASCII versus NBSP separator','leading zero and closing case'])
add('size_overflow',seq(['a<size:999999999999999999999999>b</size>z']),['integer overflow','recognized grammar versus conversion'],negative=True)
add('color_rgb',act(['a<color:#4a7>seed</color>z']),['short mixed hex','activity'])
add('color_rgba',seq(['a<color:#8044aa77>seed</color>z']),['eight digit hex via word alternative','alpha serialization'])
add('color_named_alias',note(['a<color:Grey>seed</color>z','a<color:DarkGrey>seed</color>z','a<color:#44aa77>seed</color>z']),['named aliases versus full hex','note'])
add('color_invalid_hex',seq(['a<color:#GGGGGG>seed</color>z']),['word alternative accepts invalid hex','error contract'],negative=True)
add('hidden_selection',cls(['a<b><color:~#44aa77>seed</color></b>z','a<size:~14><u>seed</u></size>z']),['hidden markers nested in bold','class-member literal fallback'])
add('font_attr_order',act(['a<font color=blue size=14 color=red size=12>seed</font>z','a<font size=12 color=red size=14 color=blue>seed</font>z']),['repeated capture retention both orders','activity'])
add('font_curly_nbsp',seq(['a<font\u00a0color=“navy”\u00a0size=“14”>seed</font>z']),['Pattern2 quote class','NBSP between attributes'])
add('font_empty_unknown',note(['a<font color=>seed</font>z','a<font size="">seed</font>z','a<font:"NeverARealFont871">seed</font>z','a<font: >seed</font>z']),['empty values and family fallback','unknown quoted family'])
add('font_hidden_precedent',wbs(['a<font color=~#44aa77>seed</font>z','a<b><font color=~#44aa77>seed</font></b>z']),['legitimate hidden family payload','nested WBS inheritance'])
add('back_forms',state(['a<back:#4a7>seed</back>z','a<back:#44aa77>seed</back>z','a<back:#8044aa77>seed</back>z','a<back:Grey>seed</back>z']),['background short/full/alpha/name grammars','state'])
add('back_gradient',note(['a<back:red-blue>seed</back>z','a<back:#44aa77/blue>seed</back>z','a<back:blue-#44aa77>seed</back>z']),['gradient second operand grammar','slash versus dash'])
add('back_hidden_invalid',act(['a<b><back:~#44aa77>seed</back></b>z','a<back:#GGGGGG>seed</back>z','a<back:>seed</back>z']),['hidden and empty backgrounds','nested activity'])
add('image_grammar',seq(['a<img src="checker-absent871.png">z','a<img:checker-absent871.png{scale=1.25}>z','a<img:checker-absent871.png{scale=+2}>z','a<img:checker-absent871.png{scale=}>z']),['src quoted versus colon','valid decimal versus invalid scale'])
add('image_nested',note(['a<b><img:checker-absent871.png{scale=2}></b>z','a<img:{bad}<color:navy>seed</color>z']),['image nested style','rejected candidate leaves inner command'])
add('icon_valid',seq(['a<&home>z','a<#blue&home{scale=1.5,color=red}>z','a<&home*2>z']),['prefix color and scale variants','valid empty lexical result'])
add('icon_invalid',state(['a<&home{scale=-2}>z','a<&home{color=~#44aa77}>z','a<&home{color=#44aa77zz}>z']),['signed scale and hidden color','invalid suffix'])
add('parameter_neighbors',seq(['a<size:14><$sprig>seed</size>[[https://example.org birch]]z','a<color:navy><&home></color><$sprig>z','a<font color=~#44aa77>seed</font><$sprig>z']),['URL sprite icon adjacency','hidden-family exception isolated'])
add('wbs_nested_parameters',wbs(['a<b><size:14><color:navy>seed</color></size></b>z','a<back:#44aa77><font size=14 color=navy>seed</font></back>z']),['nested parameter stack','WBS sibling topology'])
add('family_slash',note(['a<font:Courier/>seed</font>z','a<font:"Arial">seed</font>z']),['greedy optional slash capture','quoted family'])
add('cross_close',seq(['a<size:14><color:navy>seed</SIZE>tail</color>z','a<font size=14><b>seed</FONT>tail</b>z']),['crossed closing tags','first bounded capture'])
for name,family,consumer in [('courier_upper','COURIER',act),('courier_new','Courier New',note),('monospace_padded','" monospace "',seq),('courier_trailing','Courier ',seq),('mono','mono',state),('dialoginput','DialogInput',seq),('lucida','Lucida Console',wbs),('logical','Serif',cls),('dialog','Dialog',note),('sansserif','SansSerif',act),('monospaced_case','mOnOsPaCeD',seq),('monospace_case','MONOSPACE',wbs)]:
 add('alias_'+name,consumer(['a<font:'+family+'>Wi mi</font>z','a<b><size:14><color:navy><font:'+family+'>Wi mi</font></color></size></b>z']),['family '+repr(family),'plain versus bold/size/color nesting',consumer.__name__],alias=True)
rows=[]
for c in cases:
 p=N/(c.pop('name')+'.puml');p.write_text(c.pop('source_text'));rows.append(dict(source=str(p),cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={},**c))
(S/'manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.glob('*.puml'))]
req=json.loads((S/'maker/request.json').read_text());req.update(manifest_path=str(S/'manifest.json'),manifest_sha256=sha((S/'manifest.json').read_bytes()),case_count=len(rows),resources=resources,case_contracts=[dict(source=x['source'],source_sha256=sha(Path(x['source']).read_bytes()),cwd='<worker>/new',timezone='GMT+11:00',env_overrides={},svg_argv=['reference','--cwd','<worker>/new','--timezone','GMT+11:00','--','-pipe','-tsvg']) for x in rows])
(S/'request.json').write_text(json.dumps(req,indent=2)+'\n');files=[dict(local_path=x['local_path'],remote_relative_path=x['remote_relative_path'],sha256=x['sha256'],base64=base64.b64encode(Path(x['local_path']).read_bytes()).decode()) for x in resources]
(S/'input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(dict(request=req,manifest=rows,files=files)).encode(),mtime=0))
print(json.dumps({p.name:sha(p.read_bytes()) for p in [S/'request.json',S/'manifest.json',S/'input-bundle.json.gz']},indent=2));print(len(rows))
for p in [S/'request.json',S/'manifest.json',S/'input-bundle.json.gz',*N.glob('*.puml')]:p.chmod(0o444)
