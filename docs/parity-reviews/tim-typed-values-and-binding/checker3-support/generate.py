# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Small source-guided checker matrix; preserve every reference, including negatives."""
from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];SUP=Path(__file__).resolve().parent
rows=[]
def case(mech,name,body,axes,**kw):
 p=Path('test-diagrams/perturbations')/('tim-'+mech)/('checker3_'+name+'.puml');(ROOT/p).parent.mkdir(parents=True,exist_ok=True);(ROOT/p).write_text('@startuml\n'+body.strip()+'\n@enduml\n')
 rows.append(dict(source=str(p),mechanism='tim-'+mech,axes=axes,kind='fresh independent',**kw))
def cls(expr):return '!$result='+expr+'\nclass "$result" as Result'
for name,expr in [('substr_string','%substr("cedar", "2")'),('substr_json','%substr("cedar", %str2json("2"))'),('hex_string','%dec2hex("255")'),('hex_negative','%dec2hex(-3)'),('unicode_index','%strpos("é😀a", "a")'),('unicode_slice','%substr("é😀ab", 3, 1)')]:
 case('typed-values-and-binding',name,cls(expr),['builtin typed argument boundary','string/JSON/integer or UTF16'])
case('typed-values-and-binding','json_default', '''!$data=%str2json('["009",4]')
!function $read($a=$data, $b="008")
!return %size($a)+" / "+$b+2
!endfunction
!$data=%str2json('[1]')
!$result=$read($b="003")
class "$result" as Result''',['JSON declaration capture','named override with numeric string'])
case('typed-values-and-binding','json_index', '''!$data=%str2json('{"entry":["009",4]}')
!$result=$data.entry[0]+2
class "$result" as Result''',['JSON field and array indexing','numeric-looking string type'])
case('typed-values-and-binding','named_siblings', '''!$shared="006"
!function $leaf($a="007",$b=3)
!return $a+$b
!endfunction
!function $outer($shared="local")
!return $leaf($b=2)+" / "+%get_variable_value("$shared")
!endfunction
!$result=$outer()
class "$result" as Result''',['sibling/global versus parameter memory','named/default types'])
case('typed-values-and-binding','snapshot', '''!$counter=4
!function $advance()
!$counter=$counter+1
!return 10
!endfunction
!$result=$advance()+$counter
class "$result / $counter" as Result''',['variable token capture timing','function side effect'])
case('typed-values-and-binding','literal_macro', '''!define NEXT changed
!$word="NEXT"
!function $literal()
!return "NEXT"
!endfunction
!$joined=$literal()+"!"
class "$word / $joined" as Result''',['legacy token adapter','quoted returned text'])
for name,expr in [('wrap_mul','(1073741824 * 4) - 1'),('negative_div','(-2147483647-1) / (-1)'),('negative_quotient','(-17)/4')]:
 case('explicit-date-instant',name,cls('%date("yyyy-MM-dd HH:mm:ss", '+expr+')'),['signed int arithmetic','explicit date conversion'])
case('typed-values-and-binding','shift_scope',cls('(1 << 3)'),['operator grammar boundary','integer'],scope_probe=True)
case('typed-values-and-binding','unary_variable', '''!$n=7
!$result=(-$n)+2
class "$result" as Result''',['unary signed expression','typed variable'])
case('eager-operands-first-error','eager_recursion', '''!$trace=""
!function $walk($n)
!$trace=$trace+"x"
!if $n > 0
!return $walk($n-1)
!endif
!return 0
!endfunction
!$result=(1 || $walk(2)) && (0 || $walk(1))
class "$trace / $result" as Result''',['recursive effects','AND and OR eager operand order'])
case('eager-operands-first-error','quoted_unicode', '''!$calls=0
!function $tick()
!$calls=$calls+1
!return 0
!endfunction
!$literal="é $tick()"
!$result=(0 || $literal) && %not("")
class "$result / $calls" as Result''',['Unicode literal function text','logical truth without reparsing'])
case('eager-operands-first-error','snapshot_logical', '''!$flag=0
!function $set()
!$flag=1
!return 1
!endfunction
!$result=$set() && $flag
class "$result / $flag" as Result''',['logical eager call','variable read timing'])
case('eager-operands-first-error','fatal_substr', '''class Before
!$result=%substr("abc",-1)+%date("QQ",0)
class After''',['builtin exception before date','first error cancellation'],negative_intent=True)
case('eager-operands-first-error','fatal_unknown', '''class Before
!$result=%missing_checker3()+%date("XXXX",0)
class After''',['unknown builtin before date','first error cancellation'],negative_intent=True)
case('eager-operands-first-error','fatal_nested', '''!function $bad()
!return %date("QQ",0)
!endfunction
!function $later()
!return %date("XXXX",0)
!endfunction
class Before
!$result=$bad()+$later()
class After''',['nested function origins','multiple fatal expressions'],negative_intent=True)
case('parser-error-propagation','literal_before_date', '''class Before
!$result=%date("QQ",2147483648)
class After''',['integer literal before invalid formatter','diagnostic and source line'],negative_intent=True)
case('parser-error-propagation','default_first_error', '''!procedure $unused($a=%substr("abc",-1),$b=%date("QQ",0))
class Wrong
!endprocedure
class After''',['default evaluation exceptions','declaration-time order'],negative_intent=True)
case('function-execution-kind','recursive_plain', '''!$count=0
!procedure $visible($name)
class $name
!endprocedure
!function $walk($n)
!$count=$count+1
class Phantom
!if $n > 0
!return $walk($n-1)
!endif
$visible("Spruce")
!return "ok"
!endfunction
!$result=$walk(2)
class "$result $count" as Result''',['recursive return frames','nested emitting procedure versus plain suppression'])
case('function-execution-kind','loop_early_return', '''!$count=0
!function $once()
!while $count < 3
!$count=$count+1
!return $count
!endwhile
!return 99
!endfunction
!$result=$once()
class "$result / $count" as Result''',['return inside while','post-return mutation'])
case('function-execution-kind','foreach_return', '''!$count=0
!function $once()
!foreach $x in ["005","006"]
!$count=$count+1
!return $x
!endfor
!return "lost"
!endfunction
!$result=$once()+2
class "$result / $count" as Result''',['typed foreach values','early return termination'])
case('function-execution-kind','procedure_expression', '''!procedure $emit()
class Unexpected
!endprocedure
!$result=$emit()
class Result''',['procedure versus return call context','illegal expression invocation'],negative_intent=True)
case('function-execution-kind','no_return_extension', '''!function $emit()
class Extension
!endfunction
$emit()''',['declaration kind','documented no-return extension'],scope_probe=True)
# All file paths are root-relative and are staged identically on Monkey Man.
sub=Path('test-diagrams/perturbations/tim-external-sub-selection/checker3-lib');(ROOT/sub).mkdir(parents=True,exist_ok=True)
(ROOT/sub/'blocks.inc').write_text('!$bad=%date("QQ",0)\n!startsub chosen\n!$counter=$counter+1\n!endsub\n!startsub other\n!$bad=%date("XXXX",0)\n!endsub\n!startsub chosen\n!$counter=$counter+2\n!endsub\n!$bad=%date("QQ",0)\n')
(ROOT/sub/'tabs.inc').write_text('!startsub\tchosen\n!$counter=$counter+3\n!endsub\n')
(ROOT/sub/'nested.inc').write_text('!startsub chosen\n!$counter=$counter+1\n!startsub inner\n!$counter=$counter+2\n!endsub\n!$counter=$counter+8\n!endsub\n')
(ROOT/sub/'fatal.inc').write_text('!startsub chosen\n!$bad=%date("QQ",0)\n!$bad=%date("XXXX",0)\n!endsub\n')
(ROOT/sub/'caller.inc').write_text('!startsub chosen\n!include child.inc\n!endsub\n');(ROOT/sub/'child.inc').write_text('\n!$counter=$counter+4\n')
for name,inc in [('repeated','blocks'),('tabs','tabs'),('nested','nested'),('relative','caller')]:
 case('external-sub-selection',name,f'!$counter=0\n!includesub {sub}/{inc}.inc!chosen\nclass "$counter" as Result',['raw block selection shape '+name,'caller memory and ignored errors'])
case('external-sub-selection','dynamic_selector',f'!$counter=0\n!$which="chosen"\n!includesub {sub}/blocks.inc!$which\nclass "$counter" as Result',['evaluated selected name','ignored fatal prelude'])
case('external-sub-selection','selected_origin',f'class Before\n!includesub {sub}/fatal.inc!chosen\nclass After',['selected fatal versus ignored body','first diagnostic and file origin'],negative_intent=True)
case('function-execution-kind','include_from_return',f'''!$counter=0
!function $load()
!includesub {sub}/caller.inc!chosen
class Phantom
!return $counter
!endfunction
!$result=$load()
class "$result" as Result''',['external sub in return function','nested include and execution-kind restoration'])
for name,zone,instant in [('london_winter','Europe/London',1704067200),('london_summer','Europe/London',1719792000),('lord_howe','Australia/Lord_Howe',1704067200),('dublin_winter','Europe/Dublin',1704067200),('kathmandu','Asia/Kathmandu',0)]:
 case('zone-display-width',name,cls(f'%date("z / zzz / zzzz / zzzzz / XXX", {instant})'),['zone '+zone,'run widths and daylight/instant'],zone=zone,debug='date=1774210426000')
case('zone-display-width','fixed_fraction',cls('%date("z / zzzz / XXX", -100000)'),['fixed fractional offset','negative epoch and width'],zone='GMT-03:30',debug='date=1774210426000,tz=FIX-0330')
(SUP/'fresh-manifest.json').write_text(json.dumps(rows,indent=2)+'\n');print(len(rows))
