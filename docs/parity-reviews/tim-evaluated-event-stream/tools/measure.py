# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures
import json
import os
from pathlib import Path
import subprocess
import sys
import yaml

root = Path('/tmp/rustuml-tim-repair')
phase, suite = sys.argv[1:3]
env = dict(os.environ, PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib', RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
java = '/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
jar = '/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
if suite == 'isolated':
    sources = sorted(Path('test-diagrams/perturbations').rglob('*.puml'))
elif suite == 'preserved':
    sources = sorted(Path('test-diagrams/perturbations/tim-theme-post-expansion-source-identity').glob('*.puml'))
else:
    sources = sorted((root/'new').rglob('*.puml'))

def measure(source):
    relative = source.relative_to(root/'new') if suite == 'new' else source.relative_to('test-diagrams/perturbations')
    directory = root/suite/relative.with_suffix('')
    directory.mkdir(parents=True, exist_ok=True)
    row = {'source':str(source), 'phase':phase}
    golden = source.with_suffix('.svg')
    if suite == 'new':
        golden = directory/'java.svg'
        if not golden.exists():
            p = subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-pipe','-tsvg'],input=source.read_bytes(),capture_output=True,timeout=90)
            golden.write_bytes(p.stdout); (directory/'java.stderr').write_bytes(p.stderr)
            (directory/'java.status').write_text(str(p.returncode))
        row['java_svg_exit'] = int((directory/'java.status').read_text())
    p = subprocess.run([str(root/f'{phase}-rustuml'),'-pipe'],input=source.read_bytes(),capture_output=True,env=env,timeout=90)
    (directory/f'{phase}.svg').write_bytes(p.stdout); (directory/f'{phase}.stderr').write_bytes(p.stderr)
    row['cli_exit'] = p.returncode
    if p.returncode == 0:
        cmp = subprocess.run([str(root/'probe'),'compare',str(golden),str(directory/f'{phase}.svg')],capture_output=True,timeout=30)
        row['comparison_exit'] = cmp.returncode
        (directory/f'{phase}.diff.txt').write_bytes(cmp.stdout+cmp.stderr)
    row['result'] = 'pass' if row.get('comparison_exit') == 0 else 'fail'
    if suite != 'isolated':
        if not (directory/'java.uml-source').exists():
            p = subprocess.run([java,'-Djava.awt.headless=true','-cp',str(root)+':'+jar,'SourceProbe',str(source)],capture_output=True,timeout=90)
            (directory/'java.uml-source').write_bytes(p.stdout); (directory/'java.source.stderr').write_bytes(p.stderr)
            (directory/'java.source.status').write_text(str(p.returncode))
        row['java_source_exit'] = int((directory/'java.source.status').read_text())
        p = subprocess.run([str(root/f'{phase}-rustuml'),'--yaml','-pipe'],input=source.read_bytes(),capture_output=True,env=env,timeout=90)
        row['metadata_exit'] = p.returncode
        (directory/f'{phase}.yaml').write_bytes(p.stdout)
        if p.returncode == 0:
            node=yaml.compose(p.stdout)
            try:
                for key in ['diagram','meta','source']:
                    node={k.value:v for k,v in node.value}[key]
                text=node.value
                (directory/f'{phase}.uml-source').write_text(text)
                row['source_equal'] = text.encode()==(directory/'java.uml-source').read_bytes()
            except (KeyError,TypeError,AttributeError):
                row['source_equal'] = False
    return row

with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    results=list(pool.map(measure,sources))
(root/f'{phase}-{suite}.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps({'phase':phase,'suite':suite,'total':len(results),'pass':sum(r['result']=='pass' for r in results),'cli_error':sum(r['cli_exit']!=0 for r in results),'invalid_reference':sum(r.get('comparison_exit')==2 for r in results),'source_equal':sum(r.get('source_equal',False) for r in results)},indent=2))
