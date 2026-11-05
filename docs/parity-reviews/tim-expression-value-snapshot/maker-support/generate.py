# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json
from pathlib import Path
root=Path('/tmp/rustuml-tim4/new');root.mkdir(exist_ok=True)
cases={
'builtin_window':('tim-builtin-typed-contracts','''!$a=%substr("forest",%str2json("3"),2)
!$b=%dec2hex(-2147483648)
!$c=%dec2hex("17")
!$d=%substr("🌲ab🌿cd",2,2)
!$e=%strpos("🌲ab🌿cd","cd")
class "$a / $b / $c / $d / $e" as Window'''),
'builtin_surrogate_roundtrip':('tim-builtin-typed-contracts','''!$hi=%substr("😀",0,1)
!$lo=%substr("😀",1,1)
!$whole=$hi+$lo
!$a=%strlen($hi)
!$b=%strlen($lo)
!$c=%strlen($whole)
!$d=%strpos($whole,"😀")
class "$a / $b / $c / $d" as Units'''),
'builtin_negative_length':('tim-builtin-typed-contracts','''!$a=%substr("cedar",0,-2)
!$b=%unknown_later()
class Unreachable'''),
'snapshot_nested':('tim-expression-value-snapshot','''!$n=1
!function $advance()
!$n=$n+1
!return 1
!endfunction
!$a=%substr("abcdefgh",$advance()+$n,1)+$n
!$b=%get_variable_value("$n")
class "$a / $b" as Snapshot'''),
'snapshot_json':('tim-expression-value-snapshot','''!$data=%str2json('{"rows":["009","008"]}')
!$i=0
!function $advance()
!$i=1
!return "pick "
!endfunction
!$a=$advance()+$data.rows[$i]
!$b=$data.rows[$i]+2
class "$a / $b" as Selected'''),
'literal_define_order':('tim-literal-substitution-boundary','''!define FIRST LATER
!define LATER changed
!$word="LATER"
!function $echo()
!return "FIRST"
!endfunction
!$v=$echo()+"!"
class "FIRST / $word / $v" as Literal'''),
'literal_define_capture':('tim-literal-substitution-boundary','''!$count=0
!function $tick()
!$count=$count+1
!return "OLD"
!endfunction
!define TOKEN $tick()
!define COPY TOKEN
!define TOKEN new
!define OLD changed
class "COPY / TOKEN / $count" as Captured'''),
'return_nested':('tim-return-unwind','''!$n=0
!function $choose()
!foreach $word in ["005","006"]
!while $n < 3
!$n=$n+1
!return $word+2
!$n=99
!endwhile
!$n=88
!endfor
!return "miss"
!endfunction
!$a=$choose()
class "$a / $n" as Return'''),
'return_while_nested':('tim-return-unwind','''!$n=0
!function $choose()
!while $n < 3
!while $n < 2
!$n=$n+1
!return $n
!endwhile
!$n=90
!endwhile
!return 91
!endfunction
!$a=$choose()
class "$a / $n" as Return'''),
'procedure_value_error':('tim-return-unwind','''!procedure $emit()
class Unreachable
!endprocedure
!$v=%strlen($emit())
class After'''),
'foreach_json_global':('tim-foreach-json-binding','''!function $read()
!return $item+2
!endfunction
!function $walk($item)
!foreach $item in [3,4]
!return $item+2+" / "+$read()
!endfor
!return "miss"
!endfunction
!$v=$walk("005")
class "$v / $item" as Global'''),
'foreach_object_order':('tim-foreach-json-binding','''!$data={"z":"007","a":2}
!$text=""
!foreach $key in $data
!$text=$text+$key
!endfor
class "$text / $key" as Keys'''),
'sub_dynamic_twice':('tim-sub-directive-selection','''!$n=0
!$calls=0
!function $selector($name)
!$calls=$calls+1
!return "lib/blocks.inc!"+$name
!endfunction
!includesub $selector("chosen")
!includesub $selector("chosen")
class "$n / $calls" as Selected'''),
'sub_bad_unselected':('tim-sub-directive-selection','''!includesub lib/bad.inc!chosen
class Unreachable'''),
'sub_local_selector':('tim-sub-directive-selection','''!$n=0
!startsub\tlocal
!$n=$n+1
!endsub
!$which="local"
!includesub $which
!includesub local
class "$n" as Replayed'''),
}
(root/'lib').mkdir(exist_ok=True)
(root/'lib/blocks.inc').write_text('!$ignored=%substr("x",-1)\n!startsub\tchosen\n!$n=$n+3\n!endsub\n!$ignored=%date("QQ",0)\n')
(root/'lib/bad.inc').write_text('!startsub bad-name\n!$ignored=%date("QQ",0)\n!endsub\n!startsub chosen\nclass Selected\n!endsub\n')
rows=[]
for name,(mech,body) in cases.items():
 p=root/(name+'.puml');p.write_text('@startuml\n'+body+'\n@enduml\n')
 rows.append(dict(source=str(p),mechanism=mech,negative=name in {'builtin_negative_length','procedure_value_error','sub_bad_unselected'},zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100'))
Path('/tmp/rustuml-tim4/new-manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
