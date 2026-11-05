# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,subprocess
from pathlib import Path
S=Path('/tmp/rustuml-tim5');rows=json.loads((S/'new-manifest.json').read_bytes());results=[]
for phase in ['before','iteration1']:
 for row in rows:
  source=Path(row['source']);out=S/'new-rust'/phase/source.stem;out.mkdir(parents=True,exist_ok=True)
  env=dict(os.environ,RUSTUML_DEBUG=row['debug'],TZ=row['zone'],RAYON_NUM_THREADS='4',**row['env']);calls=[]
  for mode,args in [('svg',[]),('yaml',['--yaml'])]:
   cmd=[S/(phase+'-rustuml'),*args,'-pipe'];q=subprocess.run(cmd,input=source.read_bytes(),cwd=source.parent,env=env,capture_output=True,timeout=90)
   (out/('rust.'+mode)).write_bytes(q.stdout);(out/(mode+'.stderr')).write_bytes(q.stderr)
   calls.append(dict(argv=list(map(str,cmd)),cwd=str(source.parent),environment_overrides=dict(row['env'],RUSTUML_DEBUG=row['debug'],TZ=row['zone']),exit_code=q.returncode))
  results.append(dict(row,phase=phase,source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),calls=calls))
(S/'new-rust-results.json').write_text(json.dumps(results,indent=2)+'\n')
print('captured',len(results),'CLI/YAML pairs; Java references pending')
