# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures
import importlib.util
import json
from pathlib import Path
import yaml

path=Path(__file__).resolve().parent/'check.py'
spec=importlib.util.spec_from_file_location('check',path)
m=importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
sources=[(p,'preserved33') for p in sorted(Path('test-diagrams/perturbations/tim-theme-post-expansion-source-identity').glob('*.puml'))]
for mechanism in m.MECHANISMS:
    records=json.loads((Path('docs/parity-reviews')/mechanism/'maker-results.json').read_text())['cases']
    sources += [(Path(r['source']),'maker22') for r in records]

def measure(item):
    source,group=item
    out=m.SUPPORT/group/source.stem
    out.mkdir(parents=True,exist_ok=True)
    raw=source.read_bytes()
    (out/'input.puml').write_bytes(raw)
    j=m.run([m.JAVA,'-Djava.awt.headless=true','-jar',m.JAR,'-pipe','-tsvg'],raw)
    (out/'input.svg').write_bytes(j.stdout)
    (out/'java.stderr').write_bytes(j.stderr)
    s=m.run([m.JAVA,'-Djava.awt.headless=true','-cp',str(m.SCRATCH)+':'+m.JAR,'ReaderProbe','source',source])
    (out/'java.uml-source').write_bytes(s.stdout)
    (out/'java-source.stderr').write_bytes(s.stderr)
    r=m.run([m.BIN,'-pipe'],raw)
    (out/'rust.svg').write_bytes(r.stdout)
    (out/'rust.stderr').write_bytes(r.stderr)
    c=m.run([m.SCRATCH/'probe','compare',out/'input.svg',out/'rust.svg'])
    (out/'comparison.txt').write_bytes(c.stdout+c.stderr)
    y=m.run([m.BIN,'--yaml','-pipe'],raw)
    (out/'rust.yaml').write_bytes(y.stdout)
    node=yaml.compose(y.stdout)
    for key in ['diagram','meta','source']:
        node={k.value:v for k,v in node.value}[key]
    (out/'rust.uml-source').write_text(node.value)
    return {'source':str(source),'group':group,'java_exit':j.returncode,'cli_exit':r.returncode,'comparison_exit':c.returncode,'result':'pass' if c.returncode==r.returncode==0 else 'fail','source_equal':node.value.encode()==s.stdout}

with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    rows=list(pool.map(measure,sources))
(m.SUPPORT/'replay-results.json').write_text(json.dumps(rows,indent=2)+'\n')
for group in ['preserved33','maker22']:
    selected=[r for r in rows if r['group']==group]
    print(group,len(selected),'pass',sum(r['result']=='pass' for r in selected),'invalid',sum(r['comparison_exit']==2 for r in selected),'exact source',sum(r['source_equal'] for r in selected))
