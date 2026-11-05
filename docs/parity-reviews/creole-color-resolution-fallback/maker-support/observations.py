# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,xml.etree.ElementTree as ET
from pathlib import Path
import yaml
S=Path(__file__).parent
def canonical(node):
 if isinstance(node,yaml.MappingNode):return (node.tag, sorted((canonical(k),canonical(v)) for k,v in node.value))
 if isinstance(node,yaml.SequenceNode):return (node.tag,[canonical(v) for v in node.value])
 if isinstance(node,yaml.ScalarNode):return (node.tag,node.value)
 return None
def view(p):
 try:r=ET.fromstring(p.read_bytes())
 except ET.ParseError:return None
 return dict(canvas={k:r.get(k) for k in ['width','height','viewBox','data-diagram-type']},text=[dict(text=''.join(n.itertext()),**n.attrib) for n in r.iter() if n.tag.split('}')[-1]=='text'],rects=[dict(n.attrib) for n in r.iter() if n.tag.split('}')[-1]=='rect'])
all_rows=[];models={}
for cohort in ['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12','checker28','fresh16','golden-regressions','checker36','fallback16']:
 rows=json.loads((S/(cohort+'-membership.json')).read_bytes());before=json.loads((S/(cohort+'-before.json')).read_bytes());after=json.loads((S/(cohort+'-after.json')).read_bytes());changes=[];unparseable=[]
 for row,b,a in zip(rows,before,after):
  d0=S/'replay/before'/row['id'];d1=S/'replay/after'/row['id']
  try:
   if canonical(yaml.compose((d0/'yaml.stdout').read_bytes()))!=canonical(yaml.compose((d1/'yaml.stdout').read_bytes())):changes.append(row['id'])
  except yaml.YAMLError:unparseable.append(row['id'])
  if cohort in ['maker18','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12','checker28','fresh16','golden-regressions','checker36','fallback16'] or 'markup_' in row['id']:
   all_rows.append(dict(id=row['id'],source=row['source_capture'],reference=row['golden_capture'],valid=row['membership']['reference_valid'],before_pass=b['svg_match'],after_pass=a['svg_match'],before=view(d0/'svg.stdout'),after=view(d1/'svg.stdout'),reference_output=view(Path(row['golden_capture']))))
 models[cohort]=dict(entries=len(rows),full_yaml_semantic_changes=changes,unparseable=unparseable,raw_yaml_changes=[b['id'] for b,a in zip(before,after) if b['yaml']['stdout_sha256']!=a['yaml']['stdout_sha256']])
(S/'text-observations.json').write_text(json.dumps(all_rows,indent=2)+'\n');(S/'model-equality-observations.json').write_text(json.dumps(models,indent=2)+'\n')
print(json.dumps(models,indent=2))
