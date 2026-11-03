# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import importlib.util,json,concurrent.futures
from pathlib import Path
import yaml
spec=importlib.util.spec_from_file_location('c','/tmp/rustuml-tim-check-2fa74908/checker.py');c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
old=json.loads(Path('docs/parity-reviews/tim-evaluated-event-stream/checker-support/case-manifest.json').read_text()); rows=[dict(source=p,group='prior42') for p in old]
rows += [dict(source=str(p),group='original33') for p in sorted(Path('test-diagrams/perturbations/tim-theme-post-expansion-source-identity').glob('*.puml'))]
def one(r):
 p=Path(r['source']);out=c.S/'source-replay'/p.parent.name/p.stem;out.mkdir(parents=True,exist_ok=True)
 j=c.run([c.JAVA,'-Djava.awt.headless=true','-Duser.timezone=GMT+11:00','-cp',str(c.S)+':'+c.JAR,'ReaderProbe','source',p]);c.save(out/'java.source',j.stdout);c.save(out/'java.stderr',j.stderr)
 y=c.run([c.S/'candidate-rustuml','--yaml','-pipe'],p.read_bytes());c.save(out/'rust.yaml',y.stdout);c.save(out/'rust.stderr',y.stderr)
 r.update(java_source_exit=j.returncode,metadata_exit=y.returncode,source_equal=False)
 try:
  node=yaml.compose(y.stdout)
  for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
  c.save(out/'rust.source',node.value.encode());r['source_equal']=node.value.encode()==j.stdout
 except Exception:pass
 return r
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:result=list(pool.map(one,rows))
c.dump(c.SUP/'retained-source-replay.json',result)
for group in ['prior42','original33']:
 rs=[r for r in result if r['group']==group];print(group,len(rs),sum(r['source_equal'] for r in rs),flush=True)
# Existing counterparts are measured against original unchanged references.
rows=json.loads((c.SUP/'manifest.json').read_text())+json.loads((c.SUP/'supplement-manifest.json').read_text())
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:result=list(pool.map(lambda r:c.measure(r,'parent'),rows))
c.dump(c.SUP/'parent-additions.json',result)
print('parent additions',len(result),sum(r['svg_match'] for r in result))
