# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,gzip,hashlib,json,os,subprocess,sys
from pathlib import Path
R=Path.cwd();C=Path(__file__).parent
label=sys.argv[1];manifest=Path(sys.argv[2]);binary=C/'independent'/(label+'-bin')/'rustuml'
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
def one(item):
 body=(R/item['source']).read_bytes();row=dict(source=item['source'],source_sha256=hashlib.sha256(body).hexdigest(),results={})
 for mode,flags in [('ast',['--ast','-pipe']),('ascii',['-ttxt','-pipe']),('yaml',['--yaml','-pipe'])]:
  argv=[str(binary),*flags];p=subprocess.run(argv,input=body,capture_output=True,cwd=R,env=env,timeout=30)
  row['results'][mode]=dict(argv=argv,cwd=str(R),exit=p.returncode,stdout=p.stdout.decode(),stderr=p.stderr.decode(),stdout_sha256=hashlib.sha256(p.stdout).hexdigest())
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(one,json.loads(manifest.read_text())))
(C/(label+'-'+manifest.stem+'-models.json.gz')).write_bytes(gzip.compress(json.dumps(rows).encode(),mtime=0));print(label,len(rows))
