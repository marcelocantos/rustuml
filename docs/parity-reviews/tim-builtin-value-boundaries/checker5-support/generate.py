# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json
from pathlib import Path
S=Path(__file__).parent; D=S/'new';D.mkdir(exist_ok=True)
rows=[]
def add(name,mechanism,negative,axes,body):
 p=D/(name+'.puml');assert not p.exists();p.write_text('@startuml\n'+body+'\n@enduml\n')
 rows.append(dict(source=str(p),mechanism='tim-'+mechanism,negative=negative,axes=axes,cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
add('units_rejoin','builtin-value-boundaries',False,['UTF16 split position','upper/lower and expansion'],'''!$h=%substr("🚀",0,1)
!$l=%substr("🚀",1,1)
!$a=%strpos(%lower("AB"+$h)+%upper($l+"ß"),"🚀")
!$b=%strlen(%lower($h)+%upper($l))
!$c=%strlen(%upper("ß"+$h)+%lower($l))
class "$a/$b/$c" as Units''')
add('chr_types','builtin-value-boundaries',False,['integer/string/JSON','invalid code point'],'''!$a=%strpos(%chr(90),"Z")
!$b=%strpos(%chr("90"),%chr(0))
!$c=%strpos(%chr(%str2json("90")),%chr(0))
!$d=%strpos(%chr(1114112),%chr(-2))
class "$a/$b/$c/$d" as Codes''')
add('bool_types','builtin-value-boundaries',False,['JSON/text/number','arithmetic result type'],'''!$a=%boolval("FaLsE")+9
!$b=%boolval(%str2json("true"))+8
!$c=%boolval(0)+%boolval("1")
class "$a/$b/$c" as Truth''')
add('bool_space_error','builtin-value-boundaries',True,['whitespace','first error before arity'],'''!$a=%boolval(" true ")
!$b=%strlen()
class Lost''')
add('surrogate_display_residue','builtin-value-boundaries',True,['surrogate side','text projection'],'''!$a=%lower(%substr("🚀",1,1))
class "$a" as Residue''')
add('arity_valid_defaults','call-resolution-arity',False,['plain/expression','default/positional count'],'''!function $mix($a,$b="end")
!return $a+"/"+$b
!endfunction
!$a=$mix("fir")
!$b=%substr("quartz",2,2)
class "$a/$b/$mix($a="ash")" as Valid''')
add('arity_argument_first','call-resolution-arity',True,['nested argument failure','excess arity'],'''!$a=%strlen(1/0,"extra")
!$b=%boolval("bad")
class Lost''')
add('arity_procedure_first','call-resolution-arity',True,['call kind','missing arity'],'''!procedure $emit($a)
class Phantom
!endprocedure
!$a=$emit()
class Lost''')
add('arity_plain_named_builtin','call-resolution-arity',True,['plain builtin named key','positional coverage'],'''class "%strlen($ignored="cedar")" as Invalid''')
add('keys_exact_spelling','named-key-operands',False,['key value lacks dollar','plain versus RPN'],'''!$key="b"
!function $pair($a="first",$b="last")
!return $a+"/"+$b
!endfunction
!$r=$pair($key="violet")
class "$r/$pair($b="blue")" as Spelling''')
add('keys_local_snapshot','named-key-operands',False,['local/global keys','later mutation'],'''!$key="$a"
!function $change()
!$key="$a"
!return "plum"
!endfunction
!function $pair($a="first",$b="last")
!return $a+"/"+$b
!endfunction
!function $outer($key="$b")
!return $pair($key=$change())
!endfunction
!$r=$outer()
class "$r/$key" as Snapshot''')
add('keys_duplicate','named-key-operands',True,['duplicate key','RPN argument counting'],'''!function $pair($a="first",$b="last")
!return $a+"/"+$b
!endfunction
!$r=$pair($a="blue",$a="red")
class "$r" as Duplicate''')
add('macro_literals','original-macro-scanning',False,['value versus source','caller/body indentation'],'''!define FLAG bright
!definelong MARK(x)
 [x]
!enddefinelong
!$v="MARK(cloud)"
class Ledger {
   MARK(sky)
}
note as N
  $v / MARK(rain) / MARK / north\\nFLAG / preFLAG
end note''')
add('macro_multiline_suffix','original-macro-scanning',False,['multiline body','caller prefix/suffix'],'''!definelong LINES()
first
second
!enddefinelong
note as N
  LINES() trailing
end note''')
add('macro_argument_error','original-macro-scanning',True,['argument failure','wrong arity'],'''!definelong MARK(x)
x
!enddefinelong
class "MARK(%boolval("bad"),extra)" as Lost''')
add('sub_suffix_continuation','sub-marker-layers',False,['suffix punctuation','identifier continuation'],'''!$n=0
!startsub piece
!$n=$n+3
!endsub0
!$n=$n+4
!endsub;tail
!includesub piece
class "$n" as Kept''')
add('sub_repeated_comments','sub-marker-layers',False,['repeated selection','selected-body comments'],'''!$n=0
!includesub lib/repeated.inc!chosen
class "$n" as Repeated''')
add('sub_raw_tail_error','sub-marker-layers',True,['invalid marker after selected','raw comment'],'''!includesub lib/bad.inc!chosen
class Lost''')
add('sub_local_nested','sub-marker-layers',True,['local marker validation','nested block'],'''!startsub outer
!startsub inner
!endsub:
class Lost''')
add('json_terminal_and_skip','json-container-validation',False,['terminal missing','nonJSON versus containers'],'''!$d={"row":{"value":"008"}}
!$v=$d.row.value+2
!$m=$d.absent
!$n=0
!foreach $x in "[1,2]"
!$n=90
!endfor
!foreach $x in %str2json("broken")
!$n=80
!endfor
!foreach $x in [3,4]
!$n=$n+1
!endfor
class "$v/$m/$n/$x" as Json''')
add('json_scalar_null','json-container-validation',True,['JSON null','foreach versus nonJSON'],'''!foreach $x in %str2json("null")
class Phantom
!endfor
class Lost''')
add('json_missing_dot','json-container-validation',True,['nested prefix','dot continuation'],'''!$d={"row":{}}
!$v=$d.row.absent.next
class Lost''')
add('json_missing_index_residue','json-container-validation',True,['function in index','valid array operand'],'''!$d=["zero","one"]
!function $index()
!return 1
!endfunction
!$v=$d[$index()]
class "$v" as Residue''')
add('adjacent_dispatch','call-resolution-arity',False,['implemented variable arity','typed numeric conversions'],'''!$a=%size(%str2json('"009"'))
!$b=%size("009")
!$c=%hsl_color(0,100,50,50)
!$d=%intval("12.5")
class "$a/$b/$c/$d" as Dispatch''')
(D/'lib').mkdir(exist_ok=True)
(D/'lib/repeated.inc').write_text("!startsub chosen\n/' harmless\nignored\n'/\n!$n=$n+2\n!endsub:tail\n!startsub chosen\n!$n=$n+5\n!endsub\n")
(D/'lib/bad.inc').write_text("!startsub chosen\nclass Selected\n!endsub\n/' comment\n!startsub bad.name\n'/\n")
manifest=S/'new-manifest.json';manifest.write_text(json.dumps(rows,indent=2)+'\n')
probe=Path('docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java').resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
resources=[dict(local_path=str(p),remote_relative_path='new/'+str(p.relative_to(D)),sha256=sha(p),bytes=p.stat().st_size) for p in sorted(D.rglob('*')) if p.is_file()]
resources.append(dict(local_path=str(probe),remote_relative_path='SourceProbe.java',sha256=sha(probe),bytes=probe.stat().st_size))
request=dict(manifest_path=str(manifest),manifest_sha256=sha(manifest),case_count=len(rows),resources=resources,wrapper='/Users/marcelo/rustuml-reference/bin/reference',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',case_contracts=[dict(source=r['source'],source_sha256=sha(Path(r['source'])),cwd='<worker>/new',timezone=r['zone'],env_overrides=r['env'],svg_argv=['reference','--cwd','<worker>/new','--timezone',r['zone'],'--','-pipe','-tsvg'],probe_argv=['reference','--cwd','<worker>/new','--timezone',r['zone'],'--probe','<worker>/SourceProbe.java','--classpath','<worker>','--','<worker>/new/'+Path(r['source']).name,'<worker>/new']) for r in rows],output_contract='Preserve exact raw stdout/stderr/status for SVG and unchanged probe; source and stream SHA256, actual argv/cwd/env and pinned headless runtime provenance. No normalization or local Java.')
req=S/'remote-request.json';req.write_text(json.dumps(request,indent=2)+'\n');print(len(rows),len(resources));print('request',sha(req));print('manifest',sha(manifest))
