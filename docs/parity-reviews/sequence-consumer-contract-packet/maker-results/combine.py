# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,collections,os
from pathlib import Path
C=Path(__file__).parent
read=lambda p:json.loads(p.read_text())
for target,folders in [('before2549',['before2519','before30']),('final2549',['final2519','final30'])]:
 out=C/target;out.mkdir();rows=[]
 for folder in folders:
  for r in read(C/folder/'measurements.json'):
   r.setdefault('cohort','checker148');rows.append(r);src=C/folder/r['cli']['svg'];dst=out/src.name
   assert not dst.exists();os.link(src,dst)
 (out/'measurements.json').write_text(json.dumps(rows,indent=2)+'\n')
a=read(C/'before-all2519-models.json')+read(C/'before-maker30-models.json');b=read(C/'after-all2519-labeled-models.json')+read(C/'after-maker30-models.json');rows=[]
for x,y in zip(a,b):
 assert x['source']==y['source'] and x['source_sha256']==y['source_sha256']
 rows.append(dict(source=x['source'],source_sha256=x['source_sha256'],results={'before':x['results'],'after':y['results']},ascii_equal=all(x['results']['ascii'][k]==y['results']['ascii'][k] for k in ['stdout','stderr','exit'])))
(C/'final-model-and-ascii.json').write_text(json.dumps(rows,indent=2)+'\n');print('models',len(rows))
