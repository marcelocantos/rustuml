# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import subprocess,json,hashlib
base=Path('/tmp/rustuml-cff3/priority-fresh');base.mkdir(exist_ok=True)
cases={
'entity_self': 'entity "Mooring light" as Beacon #LightBlue\nBeacon -> Beacon : echo\n',
'entity_body': 'entity "Mooring light" as Beacon {\n+signal:int\n}\n',
'entity_nested': 'node Harbor {\nentity "Mooring light" as Beacon\n}\n',
'root_record_peer': 'record "Mooring light" as Beacon\nclass Peer\nPeer --> Beacon\n',
}
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
