# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,collections
from pathlib import Path
C=Path(__file__).parent
read=lambda p:json.loads(p.read_text())
for target,folders in [('before3107',['before3083','before24']),('final3107',['final2549','focus558'])]:
 out=C/target;out.mkdir();rows=[]
 for folder in folders:
  for r in read(C/folder/'measurements.json'):
   rows.append(r);src=C/folder/r['cli']['svg'];dst=out/src.name
   if dst.exists():assert src.read_bytes()==dst.read_bytes()
   else:os.link(src,dst)
 (out/'measurements.json').write_text(json.dumps(rows,indent=2)+'\n')
a=read(C/'before-all3107-models.json');b=read(C/'final-all3107-models.json');rows=[]
for x,y in zip(a,b):
 assert x['source']==y['source'] and x['source_sha256']==y['source_sha256']
 rows.append(dict(source=x['source'],source_sha256=x['source_sha256'],results={'before':x['results'],'after':y['results']},ascii_equal=all(x['results']['ascii'][k]==y['results']['ascii'][k] for k in ['stdout','stderr','exit'])))
(C/'final-model-and-ascii.json').write_text(json.dumps(rows,indent=2)+'\n')
a=read(C/'before3107/measurements.json');b=read(C/'final3107/measurements.json');assert len(a)==len(b)==3107
for name,lo,hi in [('frozen2549',0,2549),('checker534',2549,3083),('maker24',3083,3107)]:
 print(name,dict(collections.Counter(r['cli_comparison']['exit'] for r in a[lo:hi])),dict(collections.Counter(r['cli_comparison']['exit'] for r in b[lo:hi])))
losses=[x['source'] for x,y in zip(a,b) if x['cli_comparison']['exit']==0 and y['cli_comparison']['exit']!=0];newerrors=[x['source'] for x,y in zip(a,b) if x['cli_comparison']['exit']!=2 and x['cli']['exit']==0 and y['cli']['exit']!=0];print('LOSSES',losses,'NEW VALID ERRORS',newerrors)
