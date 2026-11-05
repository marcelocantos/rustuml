# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,gzip,base64
from pathlib import Path
S=Path(__file__).parent;N=S/'new';N.mkdir(exist_ok=True);sha=lambda b:hashlib.sha256(b).hexdigest()
seq=lambda s,skin='':'@startuml\n'+skin+'Oriole -> Finch : '+s+'\n@enduml\n'
note=lambda s,skin='':'@startuml\n'+skin+'participant Oriole\nnote right of Oriole\n'+s+'\nend note\n@enduml\n'
act=lambda s,skin='':'@startuml\n'+skin+'start\n:'+s+';\nstop\n@enduml\n'
state=lambda s,skin='':'@startuml\n'+skin+'state Perch\nPerch : '+s+'\n@enduml\n'
wbs=lambda s,skin='':'@startwbs\n'+skin+'* Nest\n** '+s+'\n@endwbs\n'
cls=lambda s,skin='':'@startuml\n'+skin+'class Perch {\n'+s+'\n}\n@enduml\n'
cases={}
def add(name,source,axes,negative=False):cases[name]=(source,axes,negative)
add('color_restored_prior',state('<color:teal>p</color><color:BadTone>q</color>r'),['closed prior color does not leak','state'])
add('color_back_cross',note('<back:orange><color:teal>a<color:bad_tone_77>b</color>c</color>d</back>e'),['cross-style nested first-close','underscore candidate'])
add('back_color_cross',act('<color:maroon><back:NoTint77>a</back>b</color>c'),['outer fill restoration','activity'])
add('back_red_inner_invalid',seq('<back:red>a<back:NoTint77>b</back>c</back>d'),['white replaces red','same-name first-close'])
add('color_heading_list',note('== <color:indigo>a<color:NoTint77>b</color>c\n* <color:teal>x<color:NoTint77>y</color>z'),['heading and list stripe','invalid inherited color'])
add('color_class_member',cls('<color:teal>a<color:NoTint77>b</color>c\n<back:NoTint77>field</back>'),['class member parser','mixed fallback commands'])
add('color_wbs_depth', '@startwbs\n* Nest\n** <color:teal>a<color:NoTint77>b</color>c\n***_ <back:NoTint77>leaf</back>\n@endwbs\n',['deeper bare WBS','invalid foreground background'])
add('color_hex_invalid',act('<color:teal>a<color:#g123ab>b</color>c<color:abc12g>d</color>e'),['prefixed versus unprefixed invalid hex','inherited fill'])
add('color_names_valid',seq('<color:DARKSLATEGREY>a</color><color:#ABCDEF>b</color><color:NoTint77>c</color>'),['case and grey alias','valid/invalid adjacent'])
add('color_numeric_names',state('<color:teal>a<color:12>b</color><color:12345>c</color><color:1234567>d</color>e'),['unsupported numeric widths','multiple invalids'])
add('color_single_nibble',seq('<color:teal>a<color:F>b</color>c'),['one-nibble valid resolver','serialization boundary'])
add('color_automatic_transparent',act('<color:automatic>a</color><color:transparent>b</color><color:background>c</color>'),['special HColor values','non-simple emission'])
add('back_gradient_axes',seq('<back:blue|yellow>a</back><back:blue/NoTint77>b</back><back:NoTint77-yellow>c</back>'),['valid vertical gradient','invalid operand order'])
add('back_gradient_hash',note('<back:#blue-#yellow>a</back><back:#NoTint77-#blue>b</back>'),['hash prefixed gradient operands','nested simple resolution'])
add('back_transparent',state('<back:red>a<back:transparent>b</back>c</back>d'),['special transparent background','outer red'])
add('font_invalid_after_valid',note('<color:teal>a<font color=NoTint77>b</font>c</color>'),['font throw under explicit fill','note'],True)
add('font_invalid_member',cls('<font color=NoTint77>field</font>'),['font command without size','class member'],True)
add('font_hex_grammar',seq('<font color=#GGHHII>a</font><font color=12>b</font>'),['malformed hex remains literal','grammatical numeric throws'],True)
img='<img:checker-ae47-absent.png>';img2='<img:checker-ae47-absent2.png>'
for family,wrap,skin in [('sequence',seq,'skinparam ArrowFontStyle bold\n'),('note',note,'skinparam NoteFontStyle bold\n'),('activity',act,'skinparam ActivityFontStyle bold\n'),('state',state,'skinparam StateFontStyle bold\n'),('class',cls,'skinparam ClassAttributeFontStyle bold\n'),('wbs',wbs,'skinparam DefaultFontStyle bold\n')]:
 for base in [False,True]:
  s='a<b><color:green>'+img+'</color></b>b<i><size:19>'+img2+'</size></i>c'
  add('image_'+family+('_base_bold' if base else '_plain_base'),wrap(s,skin if base else ''),['two missing image atoms with adjacent text',family+' explicit base bold' if base else family+' nested bold italic size color'])
add('image_sequence_base_italic',seq('a'+img+'b','skinparam ArrowFontStyle italic\n'),['base italic OR','plain image atom'])
add('image_title_default_bold','@startuml\ntitle a'+img+'b\nOriole -> Finch : ping\n@enduml\n',['title base bold','single plain fallback'])
rows=[]
for name,(source,axes,negative) in cases.items():
 p=N/(name+'.puml');p.write_text(source)
 rows.append(dict(source=str(p),mechanism='creole-image-fallback-style' if name.startswith('image_') else 'creole-color-resolution-fallback',axes=axes,cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={},negative=negative,intended_validity='intended_negative' if negative else 'intended_valid'))
(S/'manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
request=json.loads((S/'maker/request.json').read_bytes())
resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.glob('*.puml'))]
request.update(manifest_path=str(S/'manifest.json'),manifest_sha256=sha((S/'manifest.json').read_bytes()),case_count=len(rows),resources=resources,case_contracts=[dict(source=r['source'],source_sha256=sha(Path(r['source']).read_bytes()),cwd='<worker>/new',timezone='GMT+11:00',env_overrides={},svg_argv=['reference','--cwd','<worker>/new','--timezone','GMT+11:00','--','-pipe','-tsvg']) for r in rows])
(S/'request.json').write_text(json.dumps(request,indent=2)+'\n')
files=[dict(local_path=x['local_path'],remote_relative_path=x['remote_relative_path'],sha256=x['sha256'],base64=base64.b64encode(Path(x['local_path']).read_bytes()).decode()) for x in resources]
(S/'input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(dict(request=request,manifest=rows,files=files)).encode(),mtime=0))
for p in [S/'manifest.json',S/'request.json',S/'input-bundle.json.gz']:
 print(p,sha(p.read_bytes()));p.chmod(0o444)
for p in N.iterdir():p.chmod(0o444)
print('cases',len(rows))
