import json,hashlib
from pathlib import Path
S=Path('/tmp/rustuml-tim6');N=S/'new';N.mkdir(exist_ok=True)
pair='!function $join($a="oak",$b="pine")\n!return $a+"/"+$b\n!endfunction\n'
cases=[
('keys_spelling','tim-rpn-call-state',False,'!$key="b"\n'+pair+'!$x=$join($key="amber")\n!$key="$b"\n!$y=$join($key="jade")\nclass "$x/$y/$join($b="cobalt")" as Key'),
('keys_stack','tim-rpn-call-state',False,pair+'!$x="prefix"+$join($a="one",$a="two")\nclass "$x" as Stack'),
('keys_nested','tim-rpn-call-state',True,pair+'!$x=$join($a="one",$b=%upper("two"))\nclass Lost'),
('keys_duplicate','tim-rpn-call-state',True,pair+'!$x=$join($b="one",$b="two")\nclass Lost'),
('plain_named','tim-call-operand-resolution',False,'!$n=0\n!function $tick()\n!$n=$n+1\n!return "ignored"\n!endfunction\nclass "%strlen("fir",$ignored=$tick())/$n" as Arity'),
('plain_named_error','tim-call-operand-resolution',True,'class "%upper($unused="birch")" as Lost'),
('procedure_arity','tim-call-operand-resolution',True,'!procedure $emit($a)\nclass Phantom\n!endprocedure\n!$r=$emit()\nclass Lost'),
('procedure_kind','tim-call-operand-resolution',True,'!procedure $emit($a)\nclass Phantom\n!endprocedure\n!$r=$emit("x")\nclass Lost'),
('macro_raw','tim-legacy-raw-caller-prefix',False,'!$p="expanded"\n!definelong ROWS()\none\ntwo\n!enddefinelong\nnote as N\n  $p ROWS() discarded\nend note'),
('macro_effect','tim-legacy-raw-caller-prefix',False,'!$n=0\n!function $tick()\n!$n=$n+1\n!return "expanded"\n!endfunction\n!definelong ROWS()\none\ntwo\n!enddefinelong\nnote as N\n$tick() ROWS() discarded\nend note\nclass "$n" as Count'),
('sub_nested_inactive','tim-local-sub-collection',True,'!startsub outer\n!if 0\n!startsub inner\n!endif\n!endsub\nclass Lost'),
('sub_comments','tim-local-sub-collection',False,"!$n=0\n!startsub outer\n/'\n!startsub ignored\n'/\n!$n=$n+1\n!endsub\n!includesub outer\nclass \"$n\" as Count"),
('intval_values','tim-intval-conversion',False,'!$a=%intval("+007")+2\n!$b=%intval("-2147483648")\n!$c=%intval(%str2json("42"))+1\nclass "$a/$b/$c" as Integers'),
('intval_decimal','tim-intval-conversion',True,'!$a=%intval("-7.25")\n!$b=%strlen()\nclass Lost'),
('intval_overflow','tim-intval-conversion',True,'!$a=%intval("2147483648")\nclass Lost'),
('intval_space','tim-intval-conversion',True,'!$a=%intval(" 7")\nclass Lost'),
('hsl_types','tim-hsl-typed-alpha',False,'!$a=%hsl_color("240",100,50,25)\n!$b=%hsl_color(%str2json("240"),100,50,75)\n!$c=%hsl_color(240,100,50,"75")\nclass "$a/$b/$c" as Typed'),
('hsl_bounds','tim-hsl-typed-alpha',False,'!$a=%hsl_color(-90,120,35,101)\n!$b=%hsl_color(777,63,42,37)\n!$c=%hsl_color(90,-5,150,-10)\nclass "$a/$b/$c" as Bounds')]
rows=[]
for name,mech,negative,body in cases:
 p=N/(name+'.puml');assert not p.exists();p.write_text('@startuml\n'+body+'\n@enduml\n')
 rows.append(dict(source=str(p),mechanism=mech,negative=negative,cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
manifest=S/'new-manifest.json';manifest.write_text(json.dumps(rows,indent=2)+'\n')
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
probe=Path.cwd()/'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java'
resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=h(p),bytes=p.stat().st_size) for p in sorted(N.iterdir())]+[dict(local_path=str(probe),remote_relative_path=probe.name,sha256=h(probe),bytes=probe.stat().st_size)]
old=json.loads(Path('/tmp/rustuml-tim5/remote-request.json').read_text())
contracts=[]
for row in rows:
 p=Path(row['source']);c=dict(old['case_contracts'][0]);c.update(source=str(p),source_sha256=h(p));c['probe_argv']=c['probe_argv'][:-2]+['<worker>/new/'+p.name,'<worker>/new'];contracts.append(c)
request=dict(old,manifest_path=str(manifest),manifest_sha256=h(manifest),case_count=len(rows),resources=resources,case_contracts=contracts)
r=S/'remote-request.json';r.write_text(json.dumps(request,indent=2)+'\n')
print(json.dumps(dict(manifest=str(manifest),manifest_sha256=h(manifest),request=str(r),request_sha256=h(r),cases=len(rows),resources=len(resources)),indent=2))
