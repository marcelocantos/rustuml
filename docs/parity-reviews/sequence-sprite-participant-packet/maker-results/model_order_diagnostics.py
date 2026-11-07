# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,subprocess,yaml
from pathlib import Path
C=Path(__file__).parent;rows=json.loads((C/'golden-model-changes.json').read_text());env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib');results=[]
for r in rows:
 b=r['before']['results']['yaml']['stdout'];a=r['after']['results']['yaml']['stdout'];same=yaml.load(b,Loader=yaml.BaseLoader)==yaml.load(a,Loader=yaml.BaseLoader);assert same
 rec=dict(source=r['source'],changed_modes=r['modes'],yaml_value_equal=True,replays=[])
 for i in range(2):
  for mode,flag in [('model','--ast'),('yaml','--yaml')]:
   cmd=[str(C/'before-bin/rustuml'),flag,'-pipe'];p=subprocess.run(cmd,input=Path(r['source']).read_bytes(),capture_output=True,cwd=Path.cwd(),env=env,timeout=30);rec['replays'].append(dict(command=cmd,mode=mode,iteration=i,exit=p.returncode,stdout=p.stdout.decode(),stderr=p.stderr.decode(),stdout_sha256=hashlib.sha256(p.stdout).hexdigest(),differs_from_original_same_binary=p.stdout.decode()!=r['before']['results'][mode]['stdout']))
 results.append(rec)
(C/'golden-model-order-diagnostics.json').write_text(json.dumps(dict(changed_sources=len(rows),yaml_changed=sum('yaml' in r['modes'] for r in rows),ast_changed=sum('model' in r['modes'] for r in rows),ascii_changed=sum('ascii' in r['modes'] for r in rows),value_changes=0,source_cause='Unchanged HashMap fields: diagram/dot.rs attrs/defaults; DiagramMeta.sprites. Ordering is nondeterministic across fresh CLI processes. No comparator or serialized bytes were normalized; full raw captures retained.',results=results),indent=2)+'\n');print(len(rows),'all YAML values equal','same-binary ordering differences',sum(x['differs_from_original_same_binary'] for r in results for x in r['replays']))
