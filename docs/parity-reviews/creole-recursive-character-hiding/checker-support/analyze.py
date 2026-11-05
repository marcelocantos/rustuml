# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess,xml.etree.ElementTree as ET,re
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
summary=json.loads((S/'summary.json').read_bytes());cohorts=list(summary);out=[];repro=[]
for cohort in cohorts:
 m=json.loads((S/(cohort+'-membership.json')).read_bytes());b=json.loads((S/(cohort+'-before.json')).read_bytes());a=json.loads((S/(cohort+'-after.json')).read_bytes())
 for r,x,y in zip(m,b,a):
  assert r['id']==x['id']==y['id'];out.append(dict(cohort=cohort,id=r['id'],source=r['source_capture'],reference=r['golden_capture'],source_sha256=r['source_sha256'],reference_sha256=r['golden_sha256'],membership=r['membership'],before=x,after=y))
 if cohort!='checker28':
  for phase,rs in [('before',b),('after',a)]:
   prior=json.loads((S/'maker'/(cohort+'-'+phase+'.json')).read_bytes())
   for old,new in zip(prior,rs):
    for mode in ['svg','yaml']:
     if old[mode]['stdout_sha256']!=new[mode]['stdout_sha256'] or old[mode]['exit']!=new[mode]['exit']:repro.append(dict(cohort=cohort,phase=phase,id=new['id'],mode=mode,old=old[mode],new=new[mode]))
dump('all-outcomes.json',out);dump('maker-output-reproduction.json',dict(observations=1948,cli_calls=1948*4,differences=repro))
def totals(rows):
 return dict(entries=len(rows),valid=sum(x['after']['reference_valid'] for x in rows),before=sum(x['before']['svg_match'] for x in rows),after=sum(x['after']['svg_match'] for x in rows),lost_passes=[x['id'] for x in rows if x['before']['svg_match'] and not x['after']['svg_match']],new_errors=[dict(id=x['id'],mode=mode) for x in rows for mode in ['svg','yaml'] if x['before'][mode]['exit']==0 and x['after'][mode]['exit']!=0],new_panics=[x['id'] for x in rows if not x['before']['panic'] and x['after']['panic']],svg_panics=sum(x['after']['panic'] for x in rows),yaml_byte_equal=sum(x['before']['yaml']['stdout_sha256']==x['after']['yaml']['stdout_sha256'] for x in rows))
dump('totals.json',dict(replay=totals([r for r in out if r['cohort']!='checker28']),fresh=totals([r for r in out if r['cohort']=='checker28']),combined=totals(out)))
old_hashes={r['source_sha256'] for r in out if r['cohort']!='checker28'};fresh=[r for r in out if r['cohort']=='checker28'];assert not old_hashes.intersection(r['source_sha256'] for r in fresh)
golden_hashes={sha(p.read_bytes()) for p in (R/'test-diagrams/golden').rglob('*.puml')};assert len(golden_hashes)>0;assert not golden_hashes.intersection(r['source_sha256'] for r in fresh)
dump('fresh-independence.json',dict(cases=len(fresh),different_from_all_1948_replay_sources=True,golden_source_files=sum(1 for _ in (R/'test-diagrams/golden').rglob('*.puml')),golden_distinct_source_hashes=len(golden_hashes),different_from_golden_sources=True))
components=[]
for r in out:
 if r['cohort']!='checker28' and not r['id'].endswith(('html_parameters','literal_neighbors','s_shape_color_literals','s_display_commands','s_leaf_commands')):continue
 phases={}
 for phase,p in [('before',S/'replay/before'/r['id']/'svg.stdout'),('after',S/'replay/after'/r['id']/'svg.stdout'),('java',Path(r['reference']))]:
  root=ET.fromstring(p.read_bytes());elems=[dict(tag=n.tag.split('}')[-1],attrs=n.attrib,text=''.join(n.itertext()) if n.tag.split('}')[-1]=='text' else None) for n in root.iter()];phases[phase]=dict(elements=elems,text=[e for e in elems if e['tag']=='text'],links=[e for e in elems if e['tag']=='a'],canvas=root.attrib)
 phases['text_exact_after_java']=phases['after']['text']==phases['java']['text'];phases['text_count']={p:len(phases[p]['text']) for p in ['before','after','java']}
 components.append(dict(id=r['id'],before_pass=r['before']['svg_match'],after_pass=r['after']['svg_match'],**phases))
dump('component-details.json',components)
J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');jrev='71806a23780b04a5ccde2f8ceb5121edad5eb711';paths=['klimt/creole/command/CommandCreoleUrl.java','skin/rose/ComponentRoseNote.java','skin/AbstractTextualComponent.java','klimt/creole/Sea.java','klimt/creole/legacy/AtomTextUtils.java','klimt/font/FontStyle.java','url/UrlBuilder.java'];files=[]
for name in paths:
 p=J/'src/main/java/net/sourceforge/plantuml'/name;b=p.read_bytes();pin=subprocess.check_output(['git','show',jrev+':'+str(p.relative_to(J))],cwd=J);assert b==pin;files.append(dict(path=str(p.relative_to(J)),sha256=sha(b),matches_pin=True))
dump('checker-additional-java-provenance.json',dict(java_revision=jrev,files=files))
# Explicit reference classifications and immutable transport integrity.
for file,want in [('reference-results.json.gz','58bcfa1c4a5dad74a5459edefc1e927ea43bbf273651892b8741bdf3080abbd7'),('reference-transport.json','b7865602fb0425174c56922cc7c5066be613ba397efc0670eb4fe122f95f4438'),('parent-relay-input.json.gz','2c3d9373773187ba754402d1c2dfa4d6e3bcbe63a658b3bc1e9a057c689ee916')]:assert sha((S/file).read_bytes())==want
print('Totals',json.dumps(json.loads((S/'totals.json').read_bytes())));print('Maker raw output differences',len(repro));print('Fresh text exact',[(x['id'],x['text_exact_after_java'],x['text_count']) for x in components if x['id'].startswith('checker28/')])
