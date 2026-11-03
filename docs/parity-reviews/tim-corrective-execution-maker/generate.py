# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import json
root=Path('/tmp/rustuml-tim3/new');root.mkdir(exist_ok=True)
cases={
'boolean_value_kinds':('tim-typed-values-and-binding','!$bare=false\n!$integer=%false()\n!$json=%str2json("false")\n!$a=%not($bare)\n!$b=%not($integer)\n!$c=%not($json)\n!$d=%feature("dark-mode")\nclass "$a / $b / $c / $d" as Boolean'),
'kind_nested_procedure':('tim-function-execution-kind','''!$n = 0
!procedure $emit()
class Nested
!endprocedure
!function $inner()
!$n = $n + 1
class Dropped
$emit()
!return "leaf"
!endfunction
!function $outer()
ignored $inner()
!return "kept"
!endfunction
!$v = $outer()
class "$v $n" as Actual'''),
'kind_theme_repeated':('tim-function-execution-kind','''!$n = 0
!function $setup($theme)
!theme $theme
!$n = $n + 1
class Phantom
!return $n
!endfunction
!log $setup(cerulean)
!$v = $setup(cerulean)
class "count $v" as Actual'''),
'values_binding':('tim-typed-values-and-binding','''!$seed = "007"
!procedure $emit($a=$seed, $b=7, $c="c")
!$x = $a + 2
!$y = $b + 2
class "$x / $y / $c" as Values
!endprocedure
!$seed = "changed"
$emit($c="éλ", $b=8)'''),
'values_lookup_json':('tim-typed-values-and-binding','''!$j = {"n":1.5,"s":"007","b":true}
!$text = "007"
!function $relay($v)
!return $v
!endfunction
!$s = $relay(%get_variable_value("$text")) + 2
!$a = %size($relay($j))
!$b = $relay(%true()) + 2
!$c = %date("yyyy", $j)
class "$s / $a / $b / $c" as Values'''),
'values_json_scalar':('tim-typed-values-and-binding','''!$n = %str2json("1.5")
!$b = %str2json("true")
!$s = %str2json('"007"')
!function $relay($x)
!return $x
!endfunction
!$a = $relay($n) + 2
!$c = $relay($b) + 2
!$d = $relay($s) + 2
class "$a / $c / $d" as Values'''),
'values_set_global':('tim-typed-values-and-binding','''!$n = "007"
!procedure $mutate($n)
!$ignored = %set_variable_value("$n", "009")
!$x = $n + 1
class "$x" as Local
!endprocedure
$mutate("008")
!$v = %get_variable_value("$n") + 2
class "$v" as Global'''),
'values_wrap_quotient':('tim-typed-values-and-binding','''!$a = 1073741824 * 4 + 7
!$b = -2147483648 / -1
!$c = 7 / 2
!$d = "007" - 2
class "$a / $b / $c / $d" as Arithmetic'''),
'eager_and_or':('tim-eager-operands-first-error','''!$n=0
!function $tick($v)
!$n = $n * 10 + $v
!return $v
!endfunction
!$a = (0 && $tick(1)) || (1 || $tick(2))
!$b = (1 || $tick(3)) && $tick(4)
class "$n / $a / $b" as Effects'''),
'literal_truth':('tim-eager-operands-first-error','''!$n=0
!function $tick()
!$n=$n+1
!return 1
!endfunction
!$a = "$tick()"
!if $a && "0"
!$v = "ok"
!endif
class "$n / $v" as Literal'''),
'first_error_arguments':('tim-eager-operands-first-error','''!$n=0
!function $tick()
!$n=$n+1
!return "late"
!endfunction
!$v=%substr(%date("QQ",0),$tick(),%date("XXXX",0))
class Unreachable'''),
'first_error_logical':('tim-eager-operands-first-error','''!$v = 0 && %date("QQ", 0) && %date("yyyy", 2147483648)
class Unreachable'''),
'sub_caller_relative':('tim-external-sub-selection','''!$label="Caller"
!includesub lib/blocks.inc!selected
class After'''),
'sub_selected_fatal':('tim-external-sub-selection','''!includesub lib/error.inc!selected
class After'''),
'zone_tokyo_width':('tim-zone-display-width','''!$v=%date("z / zzzz / Z / XXX",0)
class "$v" as Time'''),
'zone_new_york_summer':('tim-zone-display-width','''!$v=%date("z / zzzz / Z / XXX",1593561600)
class "$v" as Time'''),
'zone_utc_width':('tim-zone-display-width','''!$v=%date("z / zzzz / Z / XXX",-1)
class "$v" as Time'''),
}
(root/'lib').mkdir(exist_ok=True)
(root/'lib/blocks.inc').write_text('!$bad=%date("QQ",0)\n!startsub selected\nclass $label\n!include child.inc\n!endsub\n!$label="Wrong"\n!startsub other\nclass Wrong\n!endsub\n!startsub selected\nclass Again\n!endsub\n')
(root/'lib/child.inc').write_text('class Child\n')
(root/'lib/error.inc').write_text('!$ignored=%date("QQ",0)\n!startsub selected\n!$bad=%date("XXXX",0)\n!endsub\n')
rows=[]
for name,(mech,body) in cases.items():
 p=root/(name+'.puml');p.write_text('@startuml\n'+body+'\n@enduml\n')
 row=dict(source=str(p),mechanism=mech,negative=name.startswith('first_error') or name=='sub_selected_fatal')
 if name.startswith('zone_'):
  row['zone']={'zone_tokyo_width':'Asia/Tokyo','zone_new_york_summer':'America/New_York','zone_utc_width':'UTC'}[name]
  row['debug']='date=1774210426000'
 rows.append(row)
Path('/tmp/rustuml-tim3/new-manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
