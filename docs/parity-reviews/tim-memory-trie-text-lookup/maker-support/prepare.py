# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-tim8-maker');N=S/'new';N.mkdir(exist_ok=True)
mem='tim-memory-trie-text-lookup';bind='tim-duplicate-formal-binding';sub='tim-empty-sub-iterator-termination'
cases=[
('stored_names',mem,False,'!$x=%set_variable_value("branch$tip","oak")\n!$x=%set_variable_value("twiǵ","ash")\n!$x=%set_variable_value("stone.key","fir")\n!$x=%set_variable_value("two words","elm")\nclass "branch$tip / twiǵ / stone.key / two words" as Names'),
('prefix_paths',mem,False,'!$x=%set_variable_value("ax","short")\n!$x=%set_variable_value("ax-bc","long")\nclass "ax-bX / ax- / ax / ax-bc / ax-bcZ" as Paths'),
('removal_paths',mem,False,'!ab="short"\n!abcde="long"\n!$before="ab / abcdX / abcde"\n!undef abcde\nclass "$before / ab / abcdX / abcde" as Removed'),
('scope_paths',mem,False,'!ab="global"\n!abcdef="long"\n!procedure $draw(ab="local",abcdeg="other")\n!undef abcdeg\nclass "ab / abcdX / abcdeg / abcdef" as Scoped\n!endprocedure\n$draw()'),
('original_boundaries',mem,False,'!$x=%set_variable_value("sap$tip","leaf")\n!$x=%set_variable_value("$tip","bud")\n!raw="sap$tip"\n!function $emit()\n!return "sap$tip"\n!endfunction\nclass "sap$tip / xsap$tip / sap$tip2 / raw / $emit() / \\nsap$tip / ##sap$tip##" as Boundaries'),
('surrogate_prefix',mem,False,'!$x=%set_variable_value("ax","short")\n!$x=%set_variable_value("ax😀z","long")\nclass "ax😁 / ax😀x / ax / ax😀z / 😀ax / ax²" as Astral'),
('duplicate_named',bind,False,'!procedure $draw(b="left",$b="dollar",b="last",$tail="end")\nclass "b / $b / $tail" as Bound\n!endprocedure\n$draw(b="named","pos",$tail="tail")'),
('duplicate_defaults',bind,False,'!$seed="007"\n!function $read($a=$seed,$a="08",$tail=2)\n!return $a+$tail\n!endfunction\n!$seed="999"\n!$v=$read()+" / "+$read("03","04",5)\nclass "$v / $read($a="05")" as Defaults'),
('duplicate_effects',bind,False,'!$count=0\n!function $tick()\n!$count=$count+1\n!return "named"\n!endfunction\n!procedure $draw($a="one",$mid="middle",$a="two")\nclass "$a / $mid / $count" as Once\n!endprocedure\n$draw($a=$tick(),"position")'),
('empty_comments',sub,False,"class Anchor\n!startsub empty\n/' discarded '/\n' discarded\n!endsub\n!$v=%intval(\"bad\")\nclass Lost"),
('empty_function',sub,False,'class Anchor\n!function $unused()\n!startsub empty\n!endsub\n!return 1\n!endfunction\nclass Lost'),
('blank_sub',sub,False,'!startsub notempty\n\n!endsub\nclass Continued'),
('empty_include',sub,False,'!include empty.iuml\nclass Tail\nHead --> Tail')]
rows=[]
for name,mech,negative,body in cases:
 p=N/(name+'.puml');assert not p.exists();p.write_text('@startuml\n'+body+'\n@enduml\n');rows.append(dict(source=str(p),mechanism=mech,negative=negative,intended_validity='valid_or_truncated_welcome' if name.startswith('empty_') else 'intended_valid',cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
(N/'empty.iuml').write_text('class Head\n!startsub empty\n!endsub\n!$v=%intval("bad")\nclass Lost\n')
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();m=S/'new-manifest.json';m.write_text(json.dumps(rows,indent=2)+'\n');old=json.loads(Path('docs/parity-reviews/tim-exact-memory-identities/maker-support/remote-request.json').read_bytes());probe=Path.cwd()/'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java'
resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=h(p),bytes=p.stat().st_size) for p in sorted(N.iterdir())]+[dict(local_path=str(probe),remote_relative_path=probe.name,sha256=h(probe),bytes=probe.stat().st_size)]
contracts=[]
for row in rows:
 p=Path(row['source']);c=dict(old['case_contracts'][0]);c.update(source=str(p),source_sha256=h(p));c['probe_argv']=c['probe_argv'][:-2]+['<worker>/new/'+p.name,'<worker>/new'];contracts.append(c)
r=S/'remote-request.json';r.write_text(json.dumps(dict(old,manifest_path=str(m),manifest_sha256=h(m),case_count=len(rows),resources=resources,case_contracts=contracts),indent=2)+'\n');print('request',h(r),'manifest',h(m),'cases',len(rows),'resources',len(resources))
