# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,json,hashlib,base64,gzip,shutil
S=pathlib.Path('/Users/marcelo/Library/Caches/rustuml-sprite-corrective-maker');N=S/'new';N.mkdir(exist_ok=True)
C=pathlib.Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker');R=pathlib.Path.cwd()
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
shutil.copy2(S/'target-before/release/rustuml',S/'before-rustuml')
for name in ['probe','probe.rs','replay.py','source-probe','source_probe.rs','standing_slice.rs']:
 shutil.copy2(C/name,S/name)
cohorts={name:json.loads((C/(name+'-membership.json')).read_bytes()) for name in ['retained','maker18','sprites50']}
cohorts['checker38']=[]
for i,o in enumerate(json.loads((C/'adjudicated-fresh-results.json').read_bytes())):
 p=pathlib.Path(o['row']['source']);cohorts['checker38'].append(dict(id='checker38/'+p.stem,index=i,suite='checker38',membership=dict(o['row'],reference_valid=o['reference_valid']),cwd=str(p.parent),source_capture=str(p),golden_capture=str(C/'fresh-reference'/p.stem/'java.svg.stdout'),classification=o['classification']))
for name,rows in cohorts.items():
 for row in rows:
  for key in ['source_capture','golden_capture','reference_source_capture']:
   if key not in row:continue
   p=pathlib.Path(row[key]);b=p.read_bytes();field=key.removesuffix('_capture')+'_sha256'
   if field in row:assert sha(b)==row[field]
   dest=S/'frozen'/row['id']/p.name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(b);row['original_'+key]=str(p);row[key]=str(dest);row[field]=sha(b)
 dump(S/(name+'-membership.json'),rows)
rows=[]
def add(name,body,axes,wbs=False):
 p=N/(name+'.puml');p.write_text(('@startwbs\n' if wbs else '@startuml\n')+body+('\n@endwbs\n' if wbs else '\n@enduml\n'));rows.append(dict(source=str(p),mechanism='sprite-corrective-shared-text',axes=axes,negative=False,intended_validity='intended_valid',cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
def seq(name,label):add(name,'Cedar -> Hazel : '+label,['command traversal','sequence present line'])
seq('nested_unknown','X<outer<$twig>>Y<<$枝>>Z')
seq('nested_escaped','X<outer~<$twig>>Y ~<$枝> Z')
seq('nested_unicode','Ω<β<$枝/葉-2>>🙂<plain>終')
seq('nested_styles','<b>X<outer<$twig>>Y</b><color:blue>Z<$leaf>Q</color>')
seq('blank_restored','top\\n\\n<size:30><color:red><$twig></color></size>\\n<size:24></size>\\nbottom')
seq('blank_small_base','top\\n\\n<$twig>\\nend')
# Base style is independently varied without changing the retained control.
p=N/'blank_small_base.puml';p.write_text(p.read_text().replace('Cedar ->','skinparam sequenceMessageFontSize 8\nCedar ->'))
add('absent_labels','Cedar -> Hazel\nHazel --> Cedar\nclass Missing',['absent labels','syntax negative retained']);rows[-1]['negative']=True;rows[-1]['intended_validity']='intended_syntax_negative'
add('absent_message','Cedar -> Hazel\nHazel --> Cedar',['absent display','no synthetic stripe'])
add('blank_message','Cedar -> Hazel :',['present empty display','blank versus absent'])
add('wbs_missing','* Root<outer<$twig>>!\n** Left<$leaf>End\n*** <$bud>\n** Plain',['command consumption','depth and empty node'],True)
add('wbs_styles','* <b>Canopy<$twig>Top</b>\n** <size:20>Branch<$leaf>End</size>\n*** Small\n** Plain',['semantic style width/height','depth and sibling placement'],True)
add('wbs_plain','* Canopy\n** Branch\n*** Leaf\n** Twig',['plain control','depth and sibling placement'],True)
add('wbs_left','* Root<$twig>End\n** Right\n*** RightLeaf\n-- <size:18>Left<$leaf>End</size>',['measured height','left-right topology'],True)
add('note_defined','sprite $sprig [2x2/16] {\nF0\n0F\n}\nclass Cedar\nnote right of Cedar\nX<$sprig>Y<$twig>Z\nend note',['defined versus missing','existing resolver control'])
add('deploy_defined','sprite $sprig [2x2/16] {\nF0\n0F\n}\nrectangle "X<$sprig>Y<$twig>Z" as Cedar',['defined versus missing','deployment resolver control'])
dump(S/'manifest.json',rows)
resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.glob('*.puml'))]
request=dict(manifest_path=str(S/'manifest.json'),manifest_sha256=sha((S/'manifest.json').read_bytes()),case_count=len(rows),resources=resources,wrapper='/Users/marcelo/rustuml-reference/bin/reference',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',case_contracts=[dict(source=c['source'],source_sha256=sha(pathlib.Path(c['source']).read_bytes()),cwd='<worker>/new',timezone='GMT+11:00',env_overrides={},svg_argv=['reference','--cwd','<worker>/new','--timezone','GMT+11:00','--','-pipe','-tsvg']) for c in rows],output_contract='Preserve exact raw stdout/stderr/status, all positive and negative outcomes, actual wrapper and derived Java argv with provenance wording, cwd/TZ/env, resource hashes and pinned headless runtime. SVG-only; no new SourceProbe required.')
dump(S/'request.json',request)
files=[dict(local_path=r['local_path'],remote_relative_path=r['remote_relative_path'],sha256=r['sha256'],base64=base64.b64encode(pathlib.Path(r['local_path']).read_bytes()).decode()) for r in resources]
(S/'input-bundle.json.gz').write_bytes(gzip.compress(json.dumps(dict(request=request,manifest=rows,files=files)).encode(),mtime=0))
dump(S/'prepare-provenance.json',dict(before_binary_sha256=sha((S/'before-rustuml').read_bytes()),request_sha256=sha((S/'request.json').read_bytes()),manifest_sha256=sha((S/'manifest.json').read_bytes()),bundle_sha256=sha((S/'input-bundle.json.gz').read_bytes()),counts={k:len(v) for k,v in cohorts.items()}))
print((S/'prepare-provenance.json').read_text())
