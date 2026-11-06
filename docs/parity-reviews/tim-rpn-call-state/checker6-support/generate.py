# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import copy, hashlib, json
from pathlib import Path
S=Path('/tmp/rustuml-checker6'); N=S/'new'; N.mkdir(exist_ok=True)
R=Path.cwd(); cases=[]
def add(name, mechanism, axes, negative, body):
    cases.append((name,'tim-'+mechanism,axes,negative,body))
pair='!function $braid($a="maple",$b="spruce")\n!return $a+"/"+$b\n!endfunction\n'
add('rpn_spelling_body','rpn-call-state',['key spelling','body session'],False,pair+'!function $outer($b="outer")\n!return $braid($b="inside")+"/"+$b\n!endfunction\n!$key="b"\n!$x=$braid($key="ignored")\n!$y=$outer($b="caller")\nclass "$x/$y" as Rpn')
add('rpn_dual_parameters','rpn-call-state',['declaration spelling','named and positional binding'],False,'!function $mix(b="plain",$b="dollar")\n!return %get_variable_value("b")+"/"+$b\n!endfunction\n!$x=$mix()\nclass "$x / $mix(b="cedar",$b="larch")" as Exact')
add('rpn_nested_consumption','rpn-call-state',['nested call','map clearing'],False,pair+'!$x=$braid($a="seed",$b=$braid("inner"))\nclass "$x" as Nested')
add('rpn_duplicate_underflow','rpn-call-state',['duplicate keys','surrounding operator'],True,pair+'!$x=7+$braid($b="one",$b="two")\nclass Lost')
add('rpn_neighbor_calls','rpn-call-state',['neighbor calls','computed key'],False,pair+'!$key="$b"\n!$x=$braid($key="elm")+"|"+$braid("ash")\nclass "$x" as Neighbor')
tick='!$n=0\n!function $tick($digit)\n!$n=$n*10+$digit\n!return "value"\n!endfunction\n'
add('operand_duplicate_effects','call-operand-resolution',['duplicate named RHS','positional order'],False,tick+'class "%upper($unused=$tick(1),"moss",$unused=$tick(2))/$n" as Effects')
add('operand_rhs_first_error','call-operand-resolution',['named RHS error','signature failure'],True,'class "%strlen($unused=%intval("4.5"))" as Lost')
add('operand_procedure_missing','call-operand-resolution',['coverage','default argument'],True,'!procedure $emit($a,$b="default")\nclass Phantom\n!endprocedure\n!$x=$emit()\nclass Lost')
add('operand_procedure_covered','call-operand-resolution',['coverage','eager operand'],True,tick+'!procedure $emit($a,$b="default")\nclass Phantom\n!endprocedure\n!$x=$emit($tick(3))\nclass Lost')
add('operand_plain_exact_shortcut','call-operand-resolution',['exact signature','unknown named key'],False,pair+'class "$braid("fir","yew",unrelated="ignored")" as Shortcut')
macro='!$p="cooked"\n!definelong STRIP(x)\nfirst x\nsecond x\n!enddefinelong\n'
add('legacy_repeated_raw','legacy-raw-caller-prefix',['prefix form','repeated invocations'],False,macro+'note as Memo\n  $p STRIP("oak") suffix\n%upper("raw") STRIP("elm") suffix\nend note')
add('legacy_procedure_control','legacy-raw-caller-prefix',['procedure contrast','prefix and suffix'],False,'!$p="cooked"\n!procedure $strip($x)\nfirst $x\nsecond $x\n!endprocedure\nnote as Memo\n  $p $strip("lime") suffix\nend note')
add('legacy_prefix_failure','legacy-raw-caller-prefix',['prefix effect error','suffix drop'],True,macro+'note as Memo\n%intval("x") STRIP("oak") suffix\nend note')
add('sub_collect_error_precedence','local-sub-collection',['buffered error','inactive nested marker'],True,'!startsub outer\n!$x=%intval("bad")\n!if 0\n!startsub inner\n!endif\n!endsub\nclass Lost')
add('sub_uncalled_function','local-sub-collection',['function collection','nested marker'],True,'!procedure $never()\n!startsub outer\n!startsub inner\n!endsub\n!endprocedure\nclass Survivor')
add('sub_comment_replay','local-sub-collection',['comment markers','replay count'],False,'!$n=1\n!startsub leaf\n/\' !startsub ignored \'/\n!$n=$n*3\n!endsub\n!includesub leaf\nclass "value $n" as Replay')
add('intval_ascii_edges','intval-conversion',['signed boundary','typed display'],False,'!$a=%intval("-000")\n!$b=%intval("+2147483647")\n!$c=%intval(%str2json("-2147483648"))\nclass "$a/$b/$c" as Edges')
add('intval_unicode_digits','intval-conversion',['Unicode digit script','sign'],False,'!$a=%intval("+１２３")\n!$b=%intval("-٤٢")\nclass "$a/$b" as Unicode')
add('intval_unicode_json','intval-conversion',['Unicode mixed digits','JSON string display'],False,'!$a=%intval(%str2json(\'"1२3"\'))\nclass "$a" as Mixed')
add('intval_invalid_sign','intval-conversion',['sign grammar','later error'],True,'!$a=%intval("+-3")+%strlen()\nclass Lost')
add('intval_underflow_bound','intval-conversion',['negative overflow','whole text'],True,'!$a=%intval("-2147483649")\nclass Lost')
add('hsl_rounding_extremes','hsl-typed-alpha',['hue float conversion','alpha rounding'],False,'!$a=%hsl_color(2147483647,51,49,1)\n!$b=%hsl_color(-2147483648,99,1,99)\n!$c=%hsl_color(361,50,50,50)\nclass "$a/$b/$c" as Rounding')
add('hsl_typed_channels','hsl-typed-alpha',['channel type','clamp'],False,'!$a=%hsl_color(120,"100",50,100)\n!$b=%hsl_color(120,100,%str2json("50"),100)\n!$c=%hsl_color(-721,101,-1,200)\nclass "$a/$b/$c" as Channels')
add('hsl_named_alpha','hsl-typed-alpha',['plain named argument','alpha type'],False,'class "%hsl_color(180,100,50,ignored=20)/%hsl_color(180,100,50,%str2json("50"))" as Alpha')
add('hsl_bad_arity','hsl-typed-alpha',['arity','operand error ordering'],True,'!$a=%hsl_color(12,%intval("bad"))\nclass Lost')
rows=[]
for name, mech, axes, neg, body in cases:
    p=N/(name+'.puml'); assert not p.exists();p.write_text('@startuml\n'+body+'\n@enduml\n')
    rows.append(dict(source=str(p),mechanism=mech,axes=axes,negative=neg,cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def dump(p,o):p.write_text(json.dumps(o,indent=2)+'\n')
m=S/'new-manifest.json';dump(m,rows)
probe=R/'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java'
old=json.loads((R/'docs/parity-reviews/tim-rpn-call-state/maker-support/remote-request.json').read_text())
resources=[dict(local_path=str(p),remote_relative_path='new/'+p.name,sha256=h(p),bytes=p.stat().st_size) for p in sorted(N.iterdir())]+[dict(local_path=str(probe),remote_relative_path=probe.name,sha256=h(probe),bytes=probe.stat().st_size)]
contracts=[]
for row in rows:
    p=Path(row['source']);c=copy.deepcopy(old['case_contracts'][0]);c.update(source=str(p),source_sha256=h(p));c['probe_argv']=c['probe_argv'][:-2]+['<worker>/new/'+p.name,'<worker>/new'];contracts.append(c)
r=S/'remote-request.json';dump(r,dict(old,manifest_path=str(m),manifest_sha256=h(m),case_count=len(rows),resources=resources,case_contracts=contracts))
print(json.dumps(dict(request=str(r),request_sha256=h(r),manifest=str(m),manifest_sha256=h(m),cases=len(rows),resources=len(resources)),indent=2))
