# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures, json, os, subprocess, sys
from pathlib import Path
import xml.etree.ElementTree as ET
import yaml
s=Path('/tmp/rustuml-review-cbd')
env=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
class Loader(yaml.SafeLoader):
    pass
def tag(loader,suffix,node):
    value=loader.construct_scalar(node) if isinstance(node,yaml.ScalarNode) else loader.construct_sequence(node) if isinstance(node,yaml.SequenceNode) else loader.construct_mapping(node)
    return {'tag':suffix,'value':value}
Loader.add_multi_constructor('!',tag)
paths=json.loads((s/'candidate-paths.json').read_text())
for name in sys.argv[1:] or ['candidate','before']:
    def one(path):
        cmd=[str(s/(name+'-rustuml')),'--yaml',str(Path(path).absolute())]
        p=subprocess.run(cmd,env=env,capture_output=True,timeout=60)
        out=s/(name+'-models')/Path(path).with_suffix('.yaml');out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(p.stdout)
        return dict(source=path,command=cmd,exit=p.returncode,stderr=p.stderr.decode(errors='replace'),model=yaml.load(p.stdout,Loader=Loader) if not p.returncode else None)
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        records=list(pool.map(one,paths))
    (s/(name+'-models.json')).write_text(json.dumps(records,indent=2)+'\n')
    print(name,len(records),flush=True)
java=[]
for path in json.loads((s/'new110.json').read_text()):
    root=ET.parse(Path(path).with_suffix('.svg')).getroot()
    groups=[dict(e.attrib) for e in root.iter() if e.tag.endswith('}g') and e.get('class') in ['entity','cluster','link']]
    java.append(dict(source=path,family=root.get('data-diagram-type'),groups=groups,text=[e.text for e in root.iter() if e.tag.endswith('}text')]))
(s/'reference-svg-inventory.json').write_text(json.dumps(java,indent=2)+'\n')
