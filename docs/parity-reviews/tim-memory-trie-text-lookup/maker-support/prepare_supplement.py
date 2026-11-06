# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-tim8-maker');N=S/'supplement';N.mkdir(exist_ok=True)
cases={'no_rescan':'!$x=%set_variable_value("twig$tip","expanded")\n!raw="twig"+"$"+"tip"\n!function $literal()\n!return "twig"+"$"+"tip"\n!endfunction\nclass "raw / $literal() / twig$tip" as Scan', 'markup_scan':'!$label="word"\n!$x=%set_variable_value("space key","value")\n!$text="<$label> / <space key> / <$missing>"\nclass "$text" as Markup'}
rows=[]
for name,body in cases.items():
 p=N/(name+'.puml');assert not p.exists();p.write_text('@startuml\n'+body+'\n@enduml\n');rows.append(dict(source=str(p),mechanism='tim-memory-trie-text-lookup',negative=False,intended_validity='intended_valid',cwd='supplement',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();m=S/'supplement-manifest.json';m.write_text(json.dumps(rows,indent=2)+'\n');old=json.loads((S/'remote-request.json').read_bytes());probe=Path.cwd()/'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java'
resources=[dict(local_path=str(p),remote_relative_path='supplement/'+p.name,sha256=h(p),bytes=p.stat().st_size) for p in sorted(N.iterdir())]+[dict(local_path=str(probe),remote_relative_path=probe.name,sha256=h(probe),bytes=probe.stat().st_size)]
contracts=[]
for row in rows:
 p=Path(row['source']);c=json.loads(json.dumps(old['case_contracts'][0]));c.update(source=str(p),source_sha256=h(p),cwd='<worker>/supplement');c['svg_argv']=[v.replace('<worker>/new','<worker>/supplement') for v in c['svg_argv']];c['probe_argv']=[v.replace('<worker>/new','<worker>/supplement') for v in c['probe_argv'][:-2]]+['<worker>/supplement/'+p.name,'<worker>/supplement'];contracts.append(c)
r=S/'supplement-request.json';r.write_text(json.dumps(dict(old,manifest_path=str(m),manifest_sha256=h(m),case_count=len(rows),resources=resources,case_contracts=contracts),indent=2)+'\n');print(h(r),h(m))
