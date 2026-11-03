# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import subprocess,json,hashlib
base=Path('/tmp/rustuml-cff3/remote-fresh');base.mkdir(exist_ok=True)
cases={}
for size in [9,14,15]:
 for kind in ['class','record']:
  cases[f'member_{kind}_{size}']=f'skinparam classAttributeIconSize {size}\npackage Arbor {{\n{kind} "Harvest ledger" as Log {{\n+fruit:int\n-hidden:int\n#branch:int\n~harvest()\n}}\n}}\n'
for kind in ['actor','rectangle','database','queue']:
 for nested in [False,True]:
  body=f'class Linden\n{kind} "Wide dispatch label\\nshort" as Relay <<service>>\n'+('Relay --> Linden\n' if nested else 'Linden --> Relay\n')
  cases[f'mixed_{kind}_{nested}']='allowmixing\n'+ ('node Grove {\n'+body+'}\n' if nested else 'left to right direction\n'+body)
for kind in ['node','cloud','database','rectangle']:
 cases[f'empty_{kind}']=f'{kind} "An empty instrument store" as Store {{\n}}\n{kind} Dock {{\n}}\nStore --> Dock\n'
for align in ['left','right']:
 cases[f'empty_ribbon_{align}']=f'{align} header\nObservatory reporting\n\nend header\nclass Linden\n'
for name,body in cases.items():
 p=base/(name+'.puml');source=('@startuml\n'+body+'@enduml\n').encode()
 if p.with_suffix('.svg').exists():
  assert p.read_bytes()==source
  continue
 p.write_bytes(source)
 cmd=['ssh','-T','-o','BatchMode=yes','marcelo@192.168.1.112','/Users/marcelo/rustuml-reference/bin/reference --cwd /Users/marcelo/rustuml-reference/scratch/worker.9DpvoRzt --timezone GMT+11:00 -- -pipe -tsvg']
 r=subprocess.run(cmd,input=source,capture_output=True,timeout=60)
 p.with_suffix('.svg').write_bytes(r.stdout);p.with_suffix('.stderr').write_bytes(r.stderr)
 p.with_suffix('.json').write_text(json.dumps(dict(command=cmd,exit=r.returncode,source_sha256=hashlib.sha256(source).hexdigest(),svg_sha256=hashlib.sha256(r.stdout).hexdigest(),java='Homebrew OpenJDK 21.0.5',graphviz='12.2.0',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',headless='forced by verified remote wrapper',timezone='GMT+11:00',transport='raw stdin bytes, no trimming',historical_equivalence_claimed=False),indent=2)+'\n')
 print(name,r.returncode,flush=True)
