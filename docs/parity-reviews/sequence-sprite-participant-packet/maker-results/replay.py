# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,hashlib,json,os,subprocess,sys,time,collections
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();rev=sys.argv[1];manifest=Path(sys.argv[2]);out=C/(rev+'-'+manifest.stem);out.mkdir();binary=C/(rev+'-bin')/'rustuml';env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib');sha=lambda b:hashlib.sha256(b).hexdigest()
def one(item):
 source=R/item['source'];body=source.read_bytes();row=dict(item,source_sha256=sha(body),results={});stem=sha(str(source).encode())[:24]
 for mode,flags in [('svg',['-tsvg','-pipe']),('ast',['--ast','-pipe']),('ascii',['-ttxt','-pipe']),('yaml',['--yaml','-pipe'])]:
  argv=[str(binary),*flags];timeout=False
  try:p=subprocess.run(argv,input=body,capture_output=True,cwd=R,env=env,timeout=30);code=p.returncode;stdout=p.stdout;stderr=p.stderr
  except subprocess.TimeoutExpired as e:code=None;stdout=e.stdout or b'';stderr=e.stderr or b'';timeout=True
  target=out/(stem+'.'+mode);target.write_bytes(stdout);row['results'][mode]=dict(argv=argv,exit=code,timeout=timeout,stdout_file=target.name,stdout_sha256=sha(stdout),stderr=stderr.decode(errors='replace'))
  if mode=='svg':
   p=subprocess.run([str(C/'before-bin/compare_cli'),str(R/item['golden']),str(target)],capture_output=True,timeout=30);row['comparison']=dict(exit=p.returncode,stdout=p.stdout.decode(errors='replace'),stderr=p.stderr.decode(errors='replace'))
 return row
start=time.monotonic()
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(one,json.loads(manifest.read_text())) )
(out/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(len(rows),collections.Counter(r['comparison']['exit'] for r in rows),time.monotonic()-start,flush=True)
