# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,re,yaml
from pathlib import Path
S=Path(__file__).resolve().parent
class Loader(yaml.SafeLoader):pass
Loader.add_multi_constructor('!',lambda loader,tag,node:{tag:loader.construct_scalar(node)})
report=[]
for scope in ['preserved','fresh']:
 b=json.load(open(S/(scope+'-before.json')))['results'];a=json.load(open(S/(scope+'-after.json')))['results']
 for old,new in zip(b,a):
  if new['yaml']['exit'] or old['yaml']['stdout_sha256']==new['yaml']['stdout_sha256']:continue
  data=yaml.load((Path(new['outputs'])/'model.yaml').read_text(),Loader=Loader)
  d=data['diagram'];s=Path(new['source'])
  if not s.is_absolute():s=Path.cwd()/s
  source=s.read_text()
  row=dict(source=new['source'],before_exit=old['yaml']['exit'],after_exit=new['yaml']['exit'],type=data['type'],uid_events=d.get('uid_events'),entities=d.get('entities'),packages=d.get('packages'),notes=d.get('notes'),relationships=d.get('relationships'),source_sha256=new['source_sha256'])
  if scope=='preserved':
   expected_entities=len(re.findall(r'^class\s',source,re.M));expected_notes=len(re.findall(r'^note\s+(?:"|as\s)',source,re.M));expected_relationships=len(re.findall(r'^\w+\s+\.\.\s+\w+',source,re.M))
   events=d['uid_events'];note_events=[v for v in events if isinstance(v,dict) and 'Note' in v]
   row['source_derived_expectations']=dict(entities=expected_entities,notes=expected_notes,relationships=expected_relationships)
   row['retention_checks']=dict(entities=len(d['entities'])==expected_entities,notes=len(d['notes'])==expected_notes,relationships=len(d['relationships'])==expected_relationships,note_uids=len(note_events)==expected_notes==1,note_url=d['notes'][0].get('url') is not None)
   assert all(row['retention_checks'].values()),row
  report.append(row)
(S/'model-checks.json').write_text(json.dumps(report,indent=2)+'\n')
print('changed successful model captures',len(report),'preserved recoveries',sum('retention_checks' in r for r in report),'all retained checks green')
