# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,subprocess,hashlib,concurrent.futures,os,sys
from pathlib import Path
cache=Path(__file__).parent;out=cache/sys.argv[1];out.mkdir(exist_ok=False)
binary='/Users/marcelo/Library/Caches/rustuml-sequence-repair/target/release/rustuml'
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
def one(row):
 p=Path(row['source']);name=p.stem; r=subprocess.run([binary,'-tsvg','-pipe'],input=p.read_bytes(),capture_output=True,env=env);actual=out/(name+'.svg');actual.write_bytes(r.stdout)
 q=subprocess.run([str(cache/'before-bin/compare_cli'),row['golden'],str(actual)],capture_output=True)
 return dict(source=str(p),cli=r.returncode,stderr=r.stderr.decode(),comparison=q.returncode,diagnostic=q.stdout.decode()+q.stderr.decode())
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool: rows=list(pool.map(one,json.loads((cache/'focused61.json').read_text())))
(out/'measurements.json').write_text(json.dumps(rows,indent=2)+'\n')
for r in rows: print(Path(r['source']).stem,r['cli'],r['comparison'],r['diagnostic'][:120])
