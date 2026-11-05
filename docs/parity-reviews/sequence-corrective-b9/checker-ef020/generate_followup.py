# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,json,hashlib
c=pathlib.Path(__file__).parent;m=json.loads((c/'reference-manifest.json').read_text());cases=[]
for engine in [0,1]:
 q=next(x for x in m['cases'] if x['name']==f'color_primitives_{engine}').copy();q['name']=f'color_primitives_valid_{engine}';q['remote_name']=q['name']+'.puml';p=c/'sources'/q['remote_name'];p.write_text(pathlib.Path(q['source']).read_text().replace(']o>',']>o'));q['source']=str(p);q['source_sha256']=hashlib.sha256(p.read_bytes()).hexdigest();q['axes'].append('CommandArrow ARROW_DRESSING2 orders head before o; original invalid input retained');cases.append(q)
 q=next(x for x in m['cases'] if x['name']==f'display_inline_{engine}').copy();q['name']=f'display_raw_link_{engine}';q['remote_name']=q['name']+'.puml';p=c/'sources'/q['remote_name'];p.write_text('@startuml\n'+('!pragma teoz true\n' if engine else '')+'Quartz -> Fern : call\nnote right : [[https://example.org/a path\\ntext]]\\nafter\\\\nlast\n@enduml\n');q['source']=str(p);q['source_sha256']=hashlib.sha256(p.read_bytes()).hexdigest();q['axes']=['raw link boundary versus following inline escape','engine/escaped backslash'];cases.append(q)
m['cases']=cases;m['purpose']='Independent source-guided follow-up: retain two invalid grammar controls and add valid primitive coverage, plus Display raw link boundary';p=c/'reference-followup-manifest.json';p.write_text(json.dumps(m,indent=2)+'\n');print(p,hashlib.sha256(p.read_bytes()).hexdigest())
