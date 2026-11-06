# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import copy,hashlib,json
from pathlib import Path
S=Path('/tmp/rustuml-checker6');N=S/'supplement';N.mkdir(exist_ok=True);R=Path.cwd()
cases=[('sub_defined_in_uncalled','tim-local-sub-collection',['function collection','external local replay'],False,'!procedure $never()\n!startsub stored\nclass Kept\n!endsub\n!endprocedure\n!includesub stored'),('operand_overload_return','tim-call-operand-resolution',['signature arity','return/procedure dispatch'],False,'!function $pick($a)\n!return "value "+$a\n!endfunction\n!procedure $pick($a,$b)\nclass Phantom\n!endprocedure\n!$x=$pick(7)\nclass "$x" as Selected'),('operand_overload_procedure','tim-call-operand-resolution',['declaration order','plain dispatch'],False,'!procedure $pick($a,$b)\nclass "$a/$b" as Kept\n!endprocedure\n!function $pick($a)\n!return "value "+$a\n!endfunction\n$pick("alder","beech")'),('rpn_bare_read','tim-rpn-call-state',['parameter identity','direct expression lookup'],False,'!function $mix(b="plain",$b="dollar")\n!return b+"/"+$b\n!endfunction\n!$x=$mix()\nclass "$x" as Read')]
rows=[]
for name,mech,axes,negative,body in cases:
 p=N/(name+'.puml');assert not p.exists();p.write_text('@startuml\n'+body+'\n@enduml\n');rows.append(dict(source=str(p),mechanism=mech,axes=axes,negative=negative,cwd='supplement',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def dump(p,o):p.write_text(json.dumps(o,indent=2)+'\n')
m=S/'supplement-manifest.json';dump(m,rows);old=json.loads((S/'remote-request.json').read_text());probe=R/'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java';resources=[dict(local_path=str(p),remote_relative_path='supplement/'+p.name,sha256=h(p),bytes=p.stat().st_size) for p in sorted(N.iterdir())]+[dict(local_path=str(probe),remote_relative_path=probe.name,sha256=h(probe),bytes=probe.stat().st_size)]
cs=[]
for row in rows:
 p=Path(row['source']);c=copy.deepcopy(old['case_contracts'][0]);c.update(source=str(p),source_sha256=h(p),cwd='<worker>/supplement');c['svg_argv']=[a.replace('<worker>/new','<worker>/supplement') for a in c['svg_argv']];c['probe_argv']=[a.replace('<worker>/new','<worker>/supplement') for a in c['probe_argv'][:-2]]+['<worker>/supplement/'+p.name,'<worker>/supplement'];cs.append(c)
r=S/'supplement-request.json';dump(r,dict(old,manifest_path=str(m),manifest_sha256=h(m),case_count=len(rows),resources=resources,case_contracts=cs));print(json.dumps(dict(request=str(r),request_sha256=h(r),manifest=str(m),manifest_sha256=h(m)),indent=2))
