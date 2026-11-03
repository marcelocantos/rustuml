# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
root=Path('/tmp/rustuml-tim-correction/new')
cases={
 'tim-expression-event-sink':{
 'nested_argument_theme':'''!$n = 0
!function $paint()
!$n = $n + 1
!theme cerulean
!return "amber"
!endfunction
!function $wrap($value)
!return "[" + $value + "]"
!endfunction
!$value = $wrap($paint())
class "$value $n" as Copper
''',
 'log_order_and_inactive':'''!$n = 0
!function $tick($word)
!$n = $n * 10 + $word
!return $word
!endfunction
!log $tick(1) $tick(2)
!if 0
!log $tick(9)
!endif
!log $tick(3)
class "Order $n" as Copper
''',
 'inline_theme_counter':'''!$n = 0
!function $paint()
!$n = $n + 1
!theme cerulean
!return $n
!endfunction
class "Paint $paint()" as Copper
''',
 'builtin_return_effect':'''!$n = 0
!function $name()
!$n = $n + 1
!theme minty
!return "$n"
!endfunction
!$exists = %variable_exists($name())
class "$exists $n" as Copper
''',
 },
 'tim-declaration-default-values':{
 'default_function_capture':'''!$n = 0
!function $once()
!$n = $n + 1
!return "a,b"
!endfunction
!procedure $emit($caption = $once())
class "$caption $n" as Copper
!endprocedure
!$n = 9
$emit()
''',
 'default_sibling_shadow':'''!$seed = "early"
!procedure $emit($caption = $seed + " suffix")
class "$caption" as Copper
!endprocedure
!procedure $call($seed)
$emit()
!endprocedure
!$seed = "late"
$call("local")
''',
 'default_explicit_override':'''!$seed = "early"
!function $label($prefix = "é,(x)", $suffix = $seed)
!return $prefix + $suffix
!endfunction
!$seed = "late"
class "$label() / $label('other', 'tail')" as Copper
''',
 },
 'tim-expression-quoted-tokens':{
 'nested_computed_lookup':'''!$needle = "silver"
!$name = "$needle"
!$a = %variable_exists(("$" + 'needle'))
!$b = %get_variable_value(%get_variable_value('$name'))
class "$a/$b" as Copper
''',
 'quoted_commas_and_unicode':'''!$needle = "value"
!function $name($ignored, $key)
!return $key
!endfunction
!$a = %get_variable_value($name('é,(x)', '$' + "needle"))
class "$a" as Copper
''',
 'literal_backslash_and_quotes':'''!function $identity($value)
!return $value
!endfunction
!$a = $identity('c:\\temp,"x"')
!$b = $identity("apostrophe's")
Alice -> Bob : $a / $b
''',
 },
 'tim-date-pattern-tokens':{
 'date_width_and_clock_fields':'''!$s = %date("y yy yyyy M MM d dd H HH h hh K k m mm s ss S SSS", 3601)
class "$s" as Copper
''',
 'date_text_and_literals':'''!$s = %date("EEEE MMMM d 'year''s day' '' 'x'", 946684800)
class "$s" as Copper
''',
 'date_zones':'''!$s = %date("Z ZZ z zzzz X XX XXX", -1)
class "$s" as Copper
''',
 'date_week_fields':'''!$s = %date("Y w W D F u EEEE", 1609372800)
class "$s" as Copper
''',
 'invalid_unclosed_pattern':'''!$s = %date("yyyy 'oops", 0)
class "$s" as Copper
''',
 'invalid_four_x':'''!$s = %date("XXXX", 0)
class "$s" as Copper
''',
 }
}
for mechanism,items in cases.items():
 for name,body in items.items():
  p=root/mechanism/(name+'.puml');p.parent.mkdir(parents=True,exist_ok=True);p.write_text('@startuml\n'+body+'@enduml\n')
