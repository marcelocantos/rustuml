# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,base64,gzip,shutil
from pathlib import Path
S=Path(__file__).parent;C=Path('/Users/marcelo/Library/Caches/rustuml-sprite-corrective-review-e09eee52');N=S/'new';N.mkdir(exist_ok=True);sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
for n in ['probe','probe.rs','replay.py']:shutil.copy2(C/n,S/n)
shutil.copy2(S/'target-before/release/rustuml',S/'before-rustuml')
p=S/'run_replay.py';p.write_text((C/'run_replay.py').read_text().replace('t3-e09eee52','t3-a295df45').replace("['checker38','maker18','sprites50','retained']","['fresh26','checker38','fresh15','maker18','sprites50','retained']"))
for name in ['retained','maker18','sprites50','checker38','fresh15','fresh26']:
 raw=(C/(name+'-membership.json')).read_bytes();(S/(name+'-original-membership.json')).write_bytes(raw);rows=json.loads(raw)
 for r in rows:
  for key in ['source_capture','golden_capture','reference_source_capture']:
   if key not in r:continue
   p=Path(r[key]);b=p.read_bytes();field=key.removesuffix('_capture')+'_sha256'
   if field in r:assert sha(b)==r[field]
   d=S/'frozen'/r['id']/p.name;d.parent.mkdir(parents=True,exist_ok=True);d.write_bytes(b);r['upstream_'+key]=str(p);r[key]=str(d);r[field]=sha(b)
 dump(S/(name+'-membership.json'),rows)
cases=[]
def add(name,body,axes,wbs=False):
 p=N/(name+'.puml');p.write_text(('@startwbs\n' if wbs else '@startuml\n')+body+('\n@endwbs\n' if wbs else '\n@enduml\n'));cases.append(dict(source=str(p),mechanism='style-wbs-command-components',axes=axes,cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={},negative=False,intended_validity='intended_valid'))
def seq(n,label):add(n,'Maple -> Alder : '+label,['supported style candidates','empty/nonempty/closing/EOL'])
seq('style_empty_families','<b></b>\\n<i></i>\\n<u></u>\\n<s></s>\\n<w></w>\\n<back:red></back>')
seq('style_empty_aliases','<del></strike>\\n<strike></s>\\n<B></b>\\n<W></w>')
seq('style_nested_candidates','A<outer<b></b>>B<$twig>C\\n<b><b></b></b>\\n<i><$bud></i>')
seq('style_parameter_contrast','<size:25></size>\\n<color:red></color>\\n<size:25><b></b></size>\\n<b>')
seq('style_escape_candidates','A<outer~<b></b>>B\\n<b>A~</b>B</b>C\\n~<i></i>')
seq('style_nonempty','A<b>Fir</b>B<i>Elm</i>C\\n<u>Birch</u><s>Ash</s><w>Oak</w>\\n<back:red>Yew</back>')
seq('style_eol','<b>Fir<$twig>End\\n<i>Elm<$bud>End\\n<s>Ash\\n<w>Oak')
add('wbs_lines','* Root\\n<$twig>\\nEnd\n** <size:20>Large</size>\\nSmall\n*** Leaf',['display lines','depth and styles'],True)
add('wbs_blank_edges','* \\nRoot\\n\n** \\n<$twig>\\n\n** Plain',['leading/trailing blanks','consumed-empty'],True)
add('wbs_escaped','* Root\\\\nLiteral\n** First\\nSecond\n** A\\\\B',['escaped backslashes','line control contrast'],True)
add('wbs_boxless_tree','*_ Root<$twig>End\n** Box\n***_ Leaf\\nNext\n--_ West',['shape root/leaf/internal','left/right and multiline'],True)
add('wbs_shape_color','* Root\n**_[#pink] East\n***[#yellow]_ Leaf\n--[#cyan]_ West',['new/old shape color order','left/deep'],True)
add('wbs_literal_underscore','* _Root\n** _ Leaf\n**_ Text',['literal underscore versus syntax','boxed/unboxed'],True)
add('wbs_boxless_empty','* Root\n**_ <$twig>\n***_ <size:20><$bud></size>\n--_ <b></b>',['unboxed empty text','style eligibility'],True)
seq('malformed_dot','A<$twig*.>B');cases[-1].update(negative=True,intended_validity='expected_java_exception')
dump(S/'manifest.json',cases)
res=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.glob('*.puml'))]
req=dict(manifest_path=str(S/'manifest.json'),manifest_sha256=sha((S/'manifest.json').read_bytes()),case_count=len(cases),resources=res,wrapper='/Users/marcelo/rustuml-reference/bin/reference',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',case_contracts=[dict(source=x['source'],source_sha256=sha(Path(x['source']).read_bytes()),cwd='<worker>/new',timezone='GMT+11:00',env_overrides={},svg_argv=['reference','--cwd','<worker>/new','--timezone','GMT+11:00','--','-pipe','-tsvg']) for x in cases],output_contract='All raw stdout/stderr/statuses including invalids, exceptions, welcome and zero bytes; exact cwd/TZ/env/wrapper argv, pinned environment, Java argv described as derived from wrapper not process trace. SVG only. No source normalization.')
dump(S/'request.json',req);files=[dict(local_path=x['local_path'],remote_relative_path=x['remote_relative_path'],sha256=x['sha256'],base64=base64.b64encode(Path(x['local_path']).read_bytes()).decode()) for x in res];(S/'input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(dict(request=req,manifest=cases,files=files)).encode(),mtime=0))
v={n:sha((S/n).read_bytes()) for n in ['request.json','manifest.json','input-bundle.json.gz','before-rustuml']};dump(S/'prepare-provenance.json',v);print(json.dumps(v,indent=2))
