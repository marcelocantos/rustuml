# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,hashlib,json,os,subprocess
from pathlib import Path
root=Path.cwd();cache=Path('/Users/marcelo/Library/Caches/rustuml-sequence-component-672')
items=json.loads((cache/'focus62.json').read_text())
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
def one(item):
 source=root/item['source'];body=source.read_bytes();row=dict(source=item['source'],source_sha256=hashlib.sha256(body).hexdigest(),results={})
 for revision,folder in [('before','before-bin'),('after','final-bin')]:
  outputs={}
  for mode,flags in [('model',['--ast','-pipe']),('ascii',['-ttxt','-pipe'])]:
   command=[str(cache/folder/'rustuml'),*flags];p=subprocess.run(command,input=body,capture_output=True,env=env,cwd=root,timeout=30)
   outputs[mode]=dict(command=command,cwd=str(root),exit=p.returncode,stdout=p.stdout.decode(),stderr=p.stderr.decode(),stdout_sha256=hashlib.sha256(p.stdout).hexdigest())
  row['results'][revision]=outputs
 row['ascii_equal']=row['results']['before']['ascii']['stdout']==row['results']['after']['ascii']['stdout']
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(one,items))
(cache/'final-model-and-ascii.json').write_text(json.dumps(rows,indent=2)+'\n')
print('entries',len(rows),'ASCII changed',sum(not x['ascii_equal'] for x in rows))
