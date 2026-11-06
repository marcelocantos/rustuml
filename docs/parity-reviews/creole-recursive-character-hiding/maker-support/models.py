# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json
from pathlib import Path
import yaml
S=Path(__file__).parent
absent={'absent':True}
def convert(n):
 if isinstance(n,yaml.MappingNode):return {k.value:convert(v) for k,v in n.value}|{'$yaml_tag':n.tag}
 if isinstance(n,yaml.SequenceNode):return [convert(v) for v in n.value]
 if isinstance(n,yaml.ScalarNode):return {'$tag':n.tag,'$value':n.value}
 return None
def diff(a,b,path=''):
 if a==b:return []
 if a==absent or b==absent:return [dict(path=path,before=a,after=b)]
 if isinstance(a,dict) and isinstance(b,dict):return [d for k in sorted(a.keys()|b.keys()) for d in diff(a.get(k,absent),b.get(k,absent),path+'/'+k)]
 if isinstance(a,list) and isinstance(b,list) and len(a)==len(b):return [d for i,(x,y) in enumerate(zip(a,b)) for d in diff(x,y,path+f'/{i}')]
 return [dict(path=path,before=a,after=b)]
rows=[]
for cohort in ['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12']:
 if not (S/(cohort+'-after.json')).exists():continue
 members=json.loads((S/(cohort+'-membership.json')).read_bytes());before=json.loads((S/(cohort+'-before.json')).read_bytes());after=json.loads((S/(cohort+'-after.json')).read_bytes());changes=[];bytechanges=[]
 for m,b,a in zip(members,before,after):
  d0=S/'replay/before'/m['id'];d1=S/'replay/after'/m['id'];x=(d0/'yaml.stdout').read_bytes();y=(d1/'yaml.stdout').read_bytes()
  if x!=y:bytechanges.append(m['id'])
  delta=diff(convert(yaml.compose(x)),convert(yaml.compose(y)))
  if delta:changes.append(dict(id=m['id'],source=m['source_capture'],changes=delta))
 rows.append(dict(cohort=cohort,entries=len(members),semantic_changes=changes,raw_byte_changes=bytechanges))
 print(cohort,'semantic models changed',len(changes),'raw changed',len(bytechanges),flush=True)
(S/'model-observations.json').write_text(json.dumps(rows,indent=2)+'\n')
