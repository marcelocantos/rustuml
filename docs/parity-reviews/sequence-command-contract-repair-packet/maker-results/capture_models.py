# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,hashlib,json,os,subprocess,sys
from pathlib import Path
R=Path.cwd();C=Path(__file__).parent;revision=sys.argv[1];manifest=Path(sys.argv[2]);bin=C/('before-bin' if revision=='before' else 'final-bin')/'rustuml'
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
def one(item):
 source=R/item['source'];body=source.read_bytes();row=dict(source=item['source'],source_sha256=hashlib.sha256(body).hexdigest(),results={})
 for mode,flags in [('model',['--ast','-pipe']),('ascii',['-ttxt','-pipe']),('yaml',['--yaml','-pipe'])]:
  command=[str(bin),*flags];p=subprocess.run(command,input=body,capture_output=True,env=env,cwd=R,timeout=30)
  row['results'][mode]=dict(command=command,cwd=str(R),exit=p.returncode,stdout=p.stdout.decode(),stderr=p.stderr.decode(),stdout_sha256=hashlib.sha256(p.stdout).hexdigest())
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool: rows=list(pool.map(one,json.loads(manifest.read_text())))
(C/(revision+'-'+manifest.stem+'-models.json')).write_text(json.dumps(rows,indent=2)+'\n');print(revision,len(rows))
