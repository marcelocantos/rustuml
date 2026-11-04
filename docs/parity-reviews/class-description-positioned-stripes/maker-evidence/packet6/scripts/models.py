# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,subprocess,yaml
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-class-cff6');B=S/'final-rustuml';OLD=Path('/Users/marcelo/Library/Caches/rustuml-class-cff5/final-rustuml')
class Loader(yaml.SafeLoader):pass
Loader.add_multi_constructor('',lambda loader,suffix,node: {suffix:loader.construct_scalar(node)} if isinstance(node,yaml.ScalarNode) else loader.construct_sequence(node) if isinstance(node,yaml.SequenceNode) else loader.construct_mapping(node))
paths=json.loads((S/'review34.json').read_text())+json.loads((S/'fresh26.json').read_text());out=S/'models';out.mkdir(exist_ok=True);rows=[]
for source in paths:
 src=Path(source); row={'source':source}
 for variant,binary in [('before',OLD),('after',B)]:
  cmd=[str(binary),'--yaml',str(src)];p=subprocess.run(cmd,capture_output=True);dest=out/(src.stem+'.'+variant+'.yaml');dest.write_bytes(p.stdout);dest.with_suffix('.stderr').write_bytes(p.stderr)
  row[variant]={'exit':p.returncode,'command':cmd,'sha256':hashlib.sha256(p.stdout).hexdigest()}
  if p.returncode:continue
  data=yaml.load(p.stdout,Loader=Loader);d=data['diagram'];row[variant]['type']=data['type'];row[variant]['counts']={k:len(d.get(k,[])) for k in ['entities','packages','notes','relationships','uid_events']};row[variant]['entities']=d.get('entities',[]);row[variant]['notes']=d.get('notes',[]);row[variant]['uid_events']=d.get('uid_events',[])
 row['unchanged_bytes']=row['before']['sha256']==row['after']['sha256'] and row['before']['exit']==row['after']['exit'];rows.append(row)
 if src.stem.startswith('url_') or src.stem.startswith('note_url_command_'):
  assert row['after']['type']=='Class';c=row['after']['counts'];assert c['entities']==1 and c['notes']==1 and c['relationships']==(0 if src.stem=='note_url_command_False' else 1)
  n=row['after']['notes'][0];assert n['url'].startswith('https://example.org/');assert n.get('url_tooltip') in ['Open chart','Inspect chart'];assert sum('Note' in str(e) for e in row['after']['uid_events'])==1
(S/'model-retention.json').write_text(json.dumps({'cases':rows,'unchanged':sum(r['unchanged_bytes'] for r in rows),'recovered':[r['source'] for r in rows if r['before']['exit'] and not r['after']['exit']]},indent=2)+'\n')
print('models',len(rows),'unchanged',sum(r['unchanged_bytes'] for r in rows))
