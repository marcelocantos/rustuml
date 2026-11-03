# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import json,re
base=Path('/tmp/rustuml-cff3');records=[]
for report in ['fresh-verified','remote-verified','priority-after']:
 rows=json.loads((base/report/'results.json').read_text())['results']
 for r in rows:
  text=(Path(r['output'])/'ast.txt').read_text();name=Path(r['relative']).stem
  counts={k:len(re.findall(r'^\s+'+k+r' \{\s*$',text,re.M)) for k in ['ClassEntity','Package','Note','Relationship','Member']}
  expected={}
  if name.startswith('native_'):expected={'ClassEntity':1,'Note':1,'Package':int(name.endswith('True'))}
  elif name.startswith('visibility_'):expected={'ClassEntity':1,'Note':1,'Package':1,'Relationship':1}
  elif name.startswith(('component_','boundary_')):expected={'ClassEntity':2,'Relationship':1}
  elif name.startswith('chrome_'):expected={'ClassEntity':1,'Package':1}
  elif name.startswith('note_frontier_'):expected={'ClassEntity':1,'Package':1,'Note':2,'Relationship':1}
  elif name.startswith('member_'):expected={'ClassEntity':1,'Package':1,'Member':4}
  elif name.startswith('mixed_'):expected={'ClassEntity':2,'Package':int(name.endswith('True')),'Relationship':1}
  elif name.startswith('empty_ribbon_'):expected={'ClassEntity':1}
  elif name.startswith('empty_'):expected={'ClassEntity':0,'Package':2,'Relationship':1}
  elif name=='entity_body':expected={'ClassEntity':1,'Member':1}
  elif name=='entity_nested':expected={'ClassEntity':1,'Package':1}
  elif name=='root_record_peer':expected={'ClassEntity':2,'Relationship':1}
  elif name=='entity_self':assert text.startswith('Sequence(') and 'Mooring light' in text and 'echo' in text
  elif name.startswith('shield_'):assert text.startswith('Component(') and 'Harbor client' in text and 'Extremely wide transport port' in text
  else:raise AssertionError(name)
  assert all(counts[k]==v for k,v in expected.items()),(name,counts,expected)
  # Check actual ID and native/member retention beyond factory selection.
  if name.startswith('member_'):assert ('kind: Record,' if '_record_' in name else 'kind: Class,') in text and 'id: "Arbor.Log"' in text
  if name.startswith('mixed_'):assert ('kind: '+name.split('_')[1].capitalize()+',') in text and 'label: "Wide dispatch label' in text
  records.append(dict(source=r['source'],source_sha256=r['source_sha256'],ast_sha256=r['ast']['stdout_sha256'],family=text.split('(')[0].strip(),expected_counts=expected,actual_counts=counts,status='maker assertion passed'))
(base/'fresh-model-assertions.json').write_text(json.dumps(records,indent=2)+'\n');print(len(records),'model probes passed')
