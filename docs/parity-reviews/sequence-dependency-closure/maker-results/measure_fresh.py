# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures, json, os, subprocess, sys
from pathlib import Path
folder=Path(sys.argv[1]); dest=Path(sys.argv[2]); binary=Path(sys.argv[3]); dest.mkdir(parents=True,exist_ok=True)
env=dict(os.environ, RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100', PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
def measure(p):
 row={'source':p.name}
 for mode,flags in [('product',['--no-oracle']),('assisted',[])]:
  r=subprocess.run([str(binary/'diff_one'),*flags,str(p)],text=True,capture_output=True,env=env,timeout=60)
  row[mode]={'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
 cli=subprocess.run([str(binary/'rustuml'),'-tsvg','-pipe'],input=p.read_text(),text=True,capture_output=True,env=env,timeout=60)
 actual=subprocess.run([str(binary/'diff_one'),'--no-oracle','--print-rust',str(p)],text=True,capture_output=True,env=env,timeout=60)
 (dest/(p.stem+'.rust.svg')).write_text(cli.stdout)
 row['cli']={'exit':cli.returncode,'stderr':cli.stderr,'same_compared_svg':cli.returncode==actual.returncode==0 and cli.stdout.strip()==actual.stdout.strip()}
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool: results=list(pool.map(measure,sorted(folder.glob('*.puml'))))
(dest/'measurements.json').write_text(json.dumps(results,indent=2)+'\n')
for mode in ('product','assisted'):
 print(mode,{code:sum(r[mode]['exit']==code for r in results) for code in sorted({r[mode]['exit'] for r in results})})
print('CLI equivalence',sum(r['cli']['same_compared_svg'] for r in results),len(results))
for r in results:
 if r['product']['exit']: print(r['source'],r['product']['exit'],r['product']['stdout'][:500])
