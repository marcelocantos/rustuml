# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib
from pathlib import Path
S=Path('/tmp/rustuml-tim5');D=S/'new';D.mkdir(exist_ok=True)
cases={
'case_units':('tim-builtin-value-boundaries',False,'''!$hi=%substr("🦉",0,1)
!$lo=%substr("🦉",1,1)
!$a=%strpos(%upper("oak"+$hi)+%lower($lo+"ASH"),"🦉")
!$b=%strlen(%upper("straße"))
!$c=%strpos(%chr("65"),"A")
!$d=%strpos(%chr(65),"A")
class "$a / $b / $c / $d" as Units'''),
'boolean_values':('tim-builtin-value-boundaries',False,'''!$a=%boolval("TrUe")+2
!$b=%boolval(%str2json("false"))+3
!$c=%boolval(1)+%boolval("0")
class "$a / $b / $c" as Boolean'''),
'boolean_error':('tim-builtin-value-boundaries',True,'''!$bad=%boolval("00")
!$later=%date("Q",0)
class Lost'''),
'arity_missing':('tim-call-resolution-arity',True,'''!$bad=%strlen()
!$later=%date("Q",0)
class Lost'''),
'arity_user_excess':('tim-call-resolution-arity',True,'''!function $only($a)
!return $a
!endfunction
!$bad=$only(1,2)
class Lost'''),
'named_global_effect':('tim-named-key-operands',False,'''!$a="$b"
!function $value()
!$a="elsewhere"
!return "oak"
!endfunction
!function $pair($a="seed",$b="leaf")
!return $a+"/"+$b
!endfunction
!$v=$pair($a=$value())
class "$v / $pair($a="elm")" as Keys'''),
'named_local':('tim-named-key-operands',False,'''!function $pair($a="root",$b="leaf")
!return $a+"/"+$b
!endfunction
!function $outer($a="$b")
!return $pair($a="ash")
!endfunction
!$v=$outer()
class "$v / $pair($a="birch")" as Local'''),
'macro_call_literals':('tim-original-macro-scanning',False,'''!definelong STAMP()
changed
!enddefinelong
!$v="STAMP()"
!function $text()
!return "STAMP()"
!endfunction
class "STAMP() / $v / $text() / STAMP" as Literal'''),
'macro_boundary_parameters':('tim-original-macro-scanning',False,'''!define TOKEN gold
!definelong WRAP(value)
[value]
!enddefinelong
!$v="WRAP(ash)"
class "WRAP(oak) / $v / north\\nTOKEN / prefixTOKEN" as Boundary'''),
'sub_suffix':('tim-sub-marker-layers',False,'''!$n=0
!startsub first
!$n=$n+1
!endsub: closing
!startsub second
!$n=$n+2
!endsub\ttrailing words
!includesub first
!includesub second
class "n=$n" as Retained'''),
'sub_selected_comments':('tim-sub-marker-layers',False,'''!$n=0
!includesub lib/blocks.inc!chosen
class "n=$n" as Selected'''),
'sub_raw_bad_comment':('tim-sub-marker-layers',True,'''!includesub lib/bad.inc!chosen
class Lost'''),
'foreach_non_json':('tim-json-container-validation',False,'''!$n=0
!foreach $x in 4
!$n=99
!endfor
!foreach $x in []
!$n=88
!endfor
!foreach $x in {"a":1,"b":2}
!$n=$n+1
!endfor
class "$n / $x" as Collection'''),
'foreach_json_boolean':('tim-json-container-validation',True,'''!foreach $x in %str2json("false")
class Lost
!endfor
class After'''),
'json_missing_traversal':('tim-json-container-validation',True,'''!$data={"ok":{}}
!$bad=$data.absent[0]
class Lost''')}
rows=[]
for name,(mech,negative,body) in cases.items():
 p=D/(name+'.puml');assert not p.exists();p.write_text('@startuml\n'+body+'\n@enduml\n');rows.append(dict(source=str(p),mechanism=mech,negative=negative,cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
(D/'lib').mkdir(exist_ok=True)
(D/'lib/blocks.inc').write_text("/' ignored prelude\nplain text\n'/\n!startsub\tchosen\n/'\nharmless selected comment\n'/\n!$n=$n+3\n!endsub ' suffix\n")
(D/'lib/bad.inc').write_text("/'\n!startsub bad-name\n'/\n!startsub chosen\nclass Lost\n!endsub\n")
manifest=S/'new-manifest.json';manifest.write_text(json.dumps(rows,indent=2)+'\n')
probe=Path('docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java').resolve()
resources=[dict(local_path=str(p),remote_relative_path='new/'+str(p.relative_to(D)),sha256=hashlib.sha256(p.read_bytes()).hexdigest(),bytes=p.stat().st_size) for p in sorted(D.rglob('*')) if p.is_file()]
resources.append(dict(local_path=str(probe),remote_relative_path='SourceProbe.java',sha256=hashlib.sha256(probe.read_bytes()).hexdigest(),bytes=probe.stat().st_size))
request=dict(manifest_path=str(manifest),manifest_sha256=hashlib.sha256(manifest.read_bytes()).hexdigest(),case_count=len(rows),resources=resources,wrapper='/Users/marcelo/rustuml-reference/bin/reference',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',case_contracts=[dict(source=r['source'],source_sha256=hashlib.sha256(Path(r['source']).read_bytes()).hexdigest(),cwd='<worker>/new',timezone=r['zone'],env_overrides=r['env'],svg_argv=['reference','--cwd','<worker>/new','--timezone',r['zone'],'--','-pipe','-tsvg'],probe_argv=['reference','--cwd','<worker>/new','--timezone',r['zone'],'--probe','<worker>/SourceProbe.java','--classpath','<worker>','--','<worker>/new/'+Path(r['source']).name,'<worker>/new']) for r in rows],output_contract='Preserve exact raw stdout/stderr/status for SVG and probe; source and stream SHA256, actual argv/cwd/env and pinned headless runtime provenance. No normalization or local Java.')
(S/'remote-request.json').write_text(json.dumps(request,indent=2)+'\n');print(len(rows),'cases',len(resources),'resources');print('request SHA256',hashlib.sha256((S/'remote-request.json').read_bytes()).hexdigest());print('manifest SHA256',request['manifest_sha256'])
