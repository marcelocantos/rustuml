# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,concurrent.futures,hashlib,json,os,subprocess,sys
from pathlib import Path
R=Path.cwd();C=Path(__file__).parent;label=sys.argv[1];manifest=Path(sys.argv[2]);binary=C/(label+'-bin')/'rustuml';items=json.loads(manifest.read_text())
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
def one(item):
 body=(R/item['source']).read_bytes();row=dict(source=item['source'],source_sha256=hashlib.sha256(body).hexdigest(),results={})
 for mode,args in [('ast',['--ast','-pipe']),('ascii',['-ttxt','-pipe']),('yaml',['--yaml','-pipe'])]:
  argv=[str(binary),*args];p=subprocess.run(argv,input=body,cwd=R,env=env,capture_output=True,timeout=30)
  row['results'][mode]=dict(argv=argv,exit=p.returncode,stdout_base64=base64.b64encode(p.stdout).decode(),stderr_base64=base64.b64encode(p.stderr).decode(),stdout_sha256=hashlib.sha256(p.stdout).hexdigest())
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(one,items))
(C/(label+'-'+manifest.stem+'-models.json')).write_text(json.dumps(rows,indent=2)+'\n');print(label,len(rows),flush=True)
