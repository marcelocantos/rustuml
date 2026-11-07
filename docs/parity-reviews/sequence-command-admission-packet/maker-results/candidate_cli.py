# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,hashlib,json,os,subprocess,sys,time,shutil
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();revision=sys.argv[1];out=C/revision;out.mkdir();binary=out/'rustuml';shutil.copy2('/Users/marcelo/Library/Caches/rustuml-sequence-repair/target/release/rustuml',binary);sha=lambda b:hashlib.sha256(b).hexdigest();(out/'binding.json').write_text(json.dumps({'binary_sha256':sha(binary.read_bytes()),'head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'source_sha256':{str(p):sha(p.read_bytes()) for p in [R/'crates/rustuml-parser/src/parse/sequence.rs',R/'crates/rustuml-render/src/sequence.rs']}}))
rows=json.loads((C/'all3561.json').read_text());rows += [dict(source=str(p.relative_to(R)),golden=str(p.with_suffix('.svg').relative_to(R)),cohort='golden-sequence') for p in sorted((R/'test-diagrams/golden/sequence').glob('*.puml'))];env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
def one(r):
 source=R/r['source'];b=source.read_bytes();cmd=[str(binary),'-tsvg','-pipe'];q=subprocess.run(cmd,input=b,capture_output=True,env=env,cwd=R,timeout=30);name=sha(r['source'].encode())[:24]+'.svg';(out/name).write_bytes(q.stdout);return dict(r,exit=q.returncode,stderr=q.stderr.decode(),stdout_file=name,stdout_sha256=sha(q.stdout),source_sha256=sha(b),command=cmd)
start=time.monotonic()
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool: results=list(pool.map(one,rows))
(out/'results.json').write_text(json.dumps(results,indent=2)+'\n');print(len(results),'errors',sum(r['exit']!=0 for r in results),'elapsed',time.monotonic()-start,flush=True)
