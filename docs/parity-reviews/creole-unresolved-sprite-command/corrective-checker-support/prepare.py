# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,json,hashlib,base64,gzip
S=pathlib.Path(__file__).parent; N=S/'new';N.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
rows=[]
def add(name,body,mechanism,axes,wbs=False,negative=False):
 p=N/(name+'.puml');p.write_text(('@startwbs\n' if wbs else '@startuml\n')+body+('\n@endwbs\n' if wbs else '\n@enduml\n'))
 rows.append(dict(source=str(p),source_sha256=sha(p.read_bytes()),mechanism=mechanism,axes=axes,negative=negative,intended_validity='intended_nonpositive' if negative else 'intended_valid',cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
t='creole-command-candidate-traversal';e='creole-present-empty-stripe';w='wbs-semantic-text-measurement'
def seq(name,label,mech=t,extra=''):
 add(name,extra+'Juniper -> Willow : '+label,mech,['command/line content','style and boundary position'])
seq('unknown_repeated','A<alien<$seed>>B<bogus<$twig>>C')
seq('unknown_known','A<alien<b>Bold</b>>B<bogus<color:red>Red</color>>C')
seq('unknown_escaped','A<alien~<$seed>>B<bogus~<b>Literal</b>>C')
seq('no_close_inner','A<alien **bold** and <$seed')
seq('unicode_edges','é<outer<$árbol>>ß<other<$葉>>ø')
seq('empty_bold_recognition','A<outer<b></b>>B<$seed>C')
seq('style_restore','<size:19>A<outer<$seed>>B</size>C<color:red>D<bogus<$bud>>E</color>F')
seq('malformed_controls','A<outer<$seed*no>>B<$seed*>C<$seed*0>D')
seq('malformed_dot','A<outer<$seed*.>>B',extra='')
rows[-1]['negative']=True;rows[-1]['intended_validity']='intended_nonpositive'
seq('present_positions',r'\nAlpha\n\nOmega\n',e)
seq('present_style_restore',r'<size:29><color:blue><$seed></color></size>\n<size:21></size>\nLast',e)
seq('present_bold_empty',r'<b></b>\n<i></i>\n<size:26><b></b></size>',e)
seq('present_small',r'\n<$seed>\n<size:30></size>\nZ',e,'skinparam sequenceMessageFontSize 7\n')
add('absent_class','class Juniper\nclass Willow\nJuniper --> Willow',e,['absent edge label','class consumer'])
add('absent_state','state Juniper\nstate Willow\nJuniper --> Willow',e,['absent edge label','state consumer'])
add('absent_activity','start\n:Juniper;\n:Willow;\nstop',e,['absent edge label','activity consumer'])
add('absent_sequence','Juniper -> Willow\nWillow --> Juniper :',e,['absent versus empty','sequence consumer'])
for name,body in [
 ('wbs_plain_depth','* Grove\n** East\n*** Twig\n**** Leaf\n-- West\n--- Bud'),
 ('wbs_mixed_depth','* <size:22>Grove</size>\n** <b>East</b>\n*** <size:17>Twig<$seed></size>\n**** Leaf\n-- <size:8>West</size>\n--- Bud'),
 ('wbs_multiline_root',r'* Grove\nCanopy'+'\n** East\n-- West'),
 ('wbs_multiline_child','* Grove\n'+r'** East\n<size:20>Branch</size>'+'\n*** Leaf\n-- West'),
 ('wbs_empty_root','* <$seed>\n** East\n-- West'),
 ('wbs_empty_children','* Grove\n** <size:24></size>\n*** <$seed>\n-- <color:red><$twig></color>'),
 ('wbs_boxless','* Grove\n**_ East<$seed>End\n*** <size:18>Leaf</size>\n--_ West'),
 ('wbs_mixed_baseline','* x<size:25>G</size>y\n** a<sub>b</sub>c\n-- <b>Wide</b>tiny'),
 ('wbs_newline_empty',r'* \nGrove\n'+'\n'+r'** East\n\nWest'),
]:add(name,body,w,['label styles/empty/multiline','root-child/depth/direction/shape'],True)
(S/'manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.glob('*.puml'))]
req=dict(manifest_path=str(S/'manifest.json'),manifest_sha256=sha((S/'manifest.json').read_bytes()),case_count=len(rows),resources=resources,wrapper='/Users/marcelo/rustuml-reference/bin/reference',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',case_contracts=[dict(source=c['source'],source_sha256=c['source_sha256'],cwd='<worker>/new',timezone='GMT+11:00',env_overrides={},svg_argv=['reference','--cwd','<worker>/new','--timezone','GMT+11:00','--','-pipe','-tsvg']) for c in rows],output_contract='SVG-only; preserve every raw stdout/stderr, exit, timeout, actual wrapper argv, private cwd, TZ/env, pinned revision/JAR and derived Java argv provenance. Return original immutable request/manifest bytes and all resources. Do not reclassify negative intent as observed validity.')
(S/'request.json').write_text(json.dumps(req,indent=2)+'\n')
files=[dict(**r,base64=base64.b64encode(pathlib.Path(r['local_path']).read_bytes()).decode()) for r in resources]
(S/'input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(dict(request=req,manifest=rows,files=files)).encode(),mtime=0))
print(len(rows),sha((S/'input-bundle.json.gz').read_bytes()))
