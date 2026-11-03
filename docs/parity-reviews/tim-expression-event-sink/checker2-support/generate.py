# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json
from pathlib import Path
S=Path('/tmp/rustuml-tim-check-2fa74908')
cases=[]
def add(m,n,body,axes,negative=False,**kw):
 p=S/'new'/m/('checker2_'+n+'.puml');p.parent.mkdir(parents=True,exist_ok=True)
 p.write_text('@startuml\n'+body+'\n@enduml\n')
 cases.append(dict(source=str(p),mechanism=m,axes=axes,negative=negative,**kw))
e='tim-expression-event-sink';d='tim-declaration-default-values';q='tim-expression-quoted-tokens';t='tim-date-pattern-tokens';x='tim-parser-error-propagation'
add(e,'return_plain_nested', '''!$count = 0
!function $tick()
!$count = $count + 1
!return "tick"
!endfunction
!function $outer()
class "discard $tick()" as Phantom
!return "visible"
!endfunction
!$label = $outer()
class "$label $count" as Actual''',['return function plain simulation','nested mutation and class count'])
add(e,'inline_procedure_suffix', '''!$count = 0
!function $tick()
!$count = $count + 1
!return "leaf"
!endfunction
!procedure $emit()
class "prefix
!endprocedure
$emit() $tick()" as Tree
class "count $count" as Count''',['procedure prefix/suffix expansion','nested function side effects'])
add(e,'log_theme_sprite', '''!$count = 0
!function $tick()
!$count = $count + 1
!theme cerulean
!return "ok"
!endfunction
!startsub glyph
sprite $bud [2x2/16] {
F1
2F
}
!endsub
!log $tick() / $tick()
class "buds $count" as Bud''',['log order and duplicate execution','subblock sprite and theme identity'])
add(e,'logical_effects', '''!$calls = 0
!function $tick()
!$calls = $calls + 1
!return 1
!endfunction
!if 1 || $tick()
class "calls $calls" as Logic
!endif''',['logical operand evaluation','function side effect'])
add(e,'literal_re_evaluation', '''!$calls = 0
!function $tick()
!$calls = $calls + 1
!return 1
!endfunction
!if "$tick()"
class "calls $calls" as Literal
!endif''',['quoted boolean token','recursive textual evaluation'])
add(e,'theme_comment_origin', '''
!function $choose()
!theme minty
!return "cerulean"
!endfunction
/' prefix '/ !theme $choose()
' removed
class Érable

class Oak
Érable --> Oak''',['nested theme order','Unicode declarations comments blanks origins'])
add(d,'captured_string_type', '''!$seed = "007"
!procedure $emit($value = $seed)
!$joined = $value + 2
class "$joined" as Captured
!endprocedure
!$seed = "99"
$emit()''',['declaration capture and later mutation','string versus numeric value type'])
add(d,'named_default', '''!$seed = "first"
!procedure $emit($a = $seed, $b = "second")
class "$a / $b" as Named
!endprocedure
!$seed = "changed"
$emit($b="named")''',['omitted first default','named argument order'])
add(d,'default_effect_order', '''!$calls = 0
!function $next()
!$calls = $calls + 1
!return $calls
!endfunction
!procedure $emit($a = $next(), $b = $next())
class "$a / $b / $calls" as Order
!endprocedure
!$calls = 8
$emit()''',['default side effect order','mutation after declaration'])
add(d,'nested_sibling_capture', '''!$seed = "global"
!procedure $emit($a = $seed)
class "$a / $seed" as Capture
!endprocedure
!procedure $outer($seed)
!local $private = "hidden"
$emit()
!endprocedure
!$seed = "new-global"
$outer("caller")''',['sibling frame isolation','captured default versus current global'])
add(q,'numeric_string_lookup', '''!$value = "007"
!$key = '$' + 'value'
!$joined = %get_variable_value($key) + 2
class "$joined" as Lookup''',['computed quoted key','string return type through builtin'])
add(q,'unicode_opposite_quotes', '''!$été = "vert"
!$key = "$" + "été"
!$v = %get_variable_value(($key))
!$literal = 'a"b,(c)'
class "$v / $literal" as Unicode''',['Unicode computed name','opposite quotes comma parentheses'])
add(q,'nested_name_side_effect', '''!$calls = 0
!$oak = "green"
!function $key()
!$calls = $calls + 1
!return '$' + 'oak'
!endfunction
!$value = %get_variable_value($key())
class "$value / $calls" as Nested''',['lookup argument function','single-quoted concatenation and count'])
add(q,'quoted_function_text', '''!$calls = 0
!function $tick()
!$calls = $calls + 1
!return "changed"
!endfunction
!$raw = '$tick()'
!$joined = $raw + ' / tail'
class "$joined / $calls" as Raw''',['literal function-like text','concatenation without reevaluation'])
for n,sec,pat in [('negative_width',-2147483648,"yyyyyyyy-MMMM-dd EEEEE DDD SSS X XX XXX"),('year_boundary',1451606400,"y Y YY YYYY w W u F 'it''s'"),('quote_unicode',-1,"yyyy 'été '' nuit' H:m:s Z"),('invalid_x',0,'XXXX'),('invalid_letter',-1,'yyyy Q'),('invalid_quote',0,"yyyy 'open")]:
 add(t,n,f'!$stamp = %date("{pat}",{sec})\nclass "$stamp" as Clock',['signed instant/date boundary','token width and quoting'],n.startswith('invalid'))
add(t,'long_zone', '!$stamp = %date("z / zzzz / Z / XXX", 0)\nclass "$stamp" as Zone',['host timezone','short versus long zone names'],zone='Australia/Melbourne',debug='date=1774210426000')
add(t,'fractional_zone', '!$stamp = %date("y-M-d H:m:s X XX XXX Z", -1)\nclass "$stamp" as Offset',['negative instant','fractional fixed offset'],zone='GMT-03:30',debug='date=1774210426000,tz=-0330')
for n,body,negative in [
 ('function_error','''!function $bad()
!return %date("QQ",0)
!endfunction
class Before
!$v = $bad()
class After''',True),
 ('default_error','''!procedure $unused($v = %date("yyyy",2147483648))
class Unused
!endprocedure
class Before''',True),
 ('inactive_error','''!if 0
!$v = %date("QQ",0)
!procedure $unused($v = %date("QQ",0))
class Never
!endprocedure
!endif
class Fine''',False),
 ('unused_function','''!function $unused()
!return %date("QQ",0)
!endfunction
class Fine''',False),
 ('sequence_error','''!procedure $bad()
Alice -> Bob: before
!$v = %date("XXXX",0)
!endprocedure
$bad()''',True),
 ('error_after_theme','''!theme cerulean
class Before
!$v = %date("yyyy Q",-1)
class After''',True),
 ('two_errors','''class Before
!$v = %date("QQ",0) + %date("yyyy",2147483648)
class After''',True),
 ('short_circuit_error','''!if 1 || %date("QQ",0)
class Before
!endif''',True),
 ('multi_block_error','''class Good
@enduml
@startuml
class Before
!$v = %date("QQ",0)''',True),
 ('include_error','''class Before
!include test-diagrams/perturbations/tim-parser-error-propagation/checker2_error_body.inc
class After''',True),
]: add(x,n,body,['error context '+n,'pattern versus overflow or inactive execution'],negative)
(S/'manifest.json').write_text(json.dumps(cases,indent=2)+'\n')
print(len(cases))
