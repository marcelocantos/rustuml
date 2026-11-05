# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Reuse preserved Java diagnostics; only the Rust CLI is executed."""
import concurrent.futures,json
from pathlib import Path
import yaml
import measure as m
s=m.S; sup=m.SUP
rows=json.loads((sup/'retained-source-replay.json').read_text())
def one(row):
 p=Path(row['source']);out=s/'outputs'/'iteration3'/'retained-source'/p.parent.name/p.stem
 r=m.run([s/'iteration3-rustuml','--yaml','-pipe'],p.read_bytes());m.save(out/'rust.yaml',r.stdout);m.save(out/'rust.stderr',r.stderr)
 result=dict(source=str(p),group=row['group'],cli_exit=r.returncode,source_equal=False)
 try:
  node=yaml.compose(r.stdout)
  for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
  m.save(out/'rust.source',node.value.encode());j=s/'checker-archive'/'source-replay'/p.parent.name/p.stem/'java.source'
  result['source_equal']=j.read_bytes()==node.value.encode()
 except Exception:pass
 return result
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:result=list(pool.map(one,rows))
m.dump(s/'retained-source-results.json',result)
for group in ['original33','prior42']:
 rs=[r for r in result if r['group']==group];print(group,len(rs),sum(r['source_equal'] for r in rs))
result=[]
for row in json.loads((sup/'file-reader-results.json').read_text()):
 p=Path(row['source']);out=s/'outputs'/'iteration3'/'file-reader'/p.stem
 r=m.run([s/'iteration3-rustuml',p]);m.save(out/'rust.svg',r.stdout);m.save(out/'rust.stderr',r.stderr)
 c=m.run(['/tmp/rustuml-tim-correction/probe','compare',p.with_suffix('.svg'),out/'rust.svg']);m.save(out/'comparison.txt',c.stdout+c.stderr)
 result.append(dict(source=str(p),cli_exit=r.returncode,svg_match=r.returncode==0 and c.returncode==0))
m.dump(s/'file-reader-results.json',result);print('file-reader',len(result),sum(r['svg_match'] for r in result))
p=Path('test-diagrams/perturbations/tim-parser-error-propagation/checker2_multi_block_error.puml');r=m.run([s/'iteration3-rustuml','--block=1','-pipe'],p.read_bytes());out=s/'outputs'/'iteration3'/'multi-block-selected';m.save(out/'stdout',r.stdout);m.save(out/'stderr',r.stderr);m.dump(s/'multi-block-selected.json',dict(source=str(p),cli_exit=r.returncode,stdout_bytes=len(r.stdout),stderr=r.stderr.decode()))
