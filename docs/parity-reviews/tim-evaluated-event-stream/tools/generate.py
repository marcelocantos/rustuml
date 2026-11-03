# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
root=Path('/tmp/rustuml-tim-repair/new')
cases={
 'tim-evaluated-event-stream':{
 'sprite_before_dynamic_theme':'''@startuml
sprite $badge [3x2/16] {
F0A
01F
}
!$palette = "cerulean"
!theme $palette
!dump_memory
class SapphireRegistry
@enduml
''',
 'sub_state_two_mutations':'''@startuml
!$counter = 0
!startsub prime
!$counter = $counter + 1
!endsub
!theme cerulean
!startsub tint
!$counter = $counter + 1
skinparam classBorderColor #A41367
!endsub
class "Ordinal $counter" as CobaltArchive
@enduml
''',
 'inactive_and_comments':'''@startuml

/' prefix '/ !theme cerulean
!if 0
!theme minty
!option handwritten true
!theme united
!endif
class CobaltRegistry /' suffix '/
@enduml
''',
 'function_selector_parameter':'''@startuml
!function $palette($choice)
!return $choice
!endfunction
!theme $palette("minty")
sprite $rune [1x3/16] {
A
B
C
}
class JasperIndex
@enduml
''',
 'sub_sprite_after_theme':'''@startuml
!theme cerulean
!startsub badge
sprite $badge [2x3/16] {
AF
F0
0A
}
!endsub
class ObsidianLedger
@enduml
''',
 'loop_side_effect':'''@startuml
!$count = 0
!procedure $choose()
!$count = $count + 1
!theme cerulean
!endprocedure
$choose()
class "Executions $count" as ZirconRecord
@enduml
'''},
 'tim-procedure-global-fork-memory':{
 'nested_local_and_global':'''@startuml
!$shared = "outer"
!procedure $inner()
class "Inner $shared $private" as InnerRecord
!endprocedure
!procedure $outer($shared)
!$private = "secret"
$inner()
class "Outer $shared $private" as OuterRecord
!endprocedure
$outer("parameter")
class "Global $shared" as GlobalRecord
@enduml
''',
 'explicit_global_shadow':'''@startuml
!$tone = "initial"
!procedure $update($tone)
!local $private = "local"
!global $tone = "updated"
class "Local $tone $private" as LocalRecord
!endprocedure
$update("parameter")
class "Global $tone" as GlobalRecord
@enduml
''',
 'theme_scope_helper':'''@startuml
!procedure $theme_setup()
!theme minty
!endprocedure
$theme_setup()
class OpalRegistry
@enduml
'''},
 'tim-literal-variable-lookup':{
 'guard_changed_theme':'''@startuml
!$BGCOLOR = "#721345"
!theme minty-outline
class GarnetRegistry
@enduml
''',
 'computed_local_name':'''@startuml
!procedure $exists($value)
!$name = "$" + "value"
!if %variable_exists($name)
class PresentRecord
!endif
!if %variable_exists("value")
class WrongBareRecord
!endif
!if %variable_exists("$absent")
class WrongAbsentRecord
!endif
!endprocedure
$exists("payload")
@enduml
'''},
 'tim-explicit-date-instant':{
 'negative_and_zero':'''@startuml
!theme cerulean
!$older = %date("yyyy-MM-dd", -86400)
!$epoch = %date("yyyy", 0)
class "$older / $epoch" as OnyxClock
@enduml
''',
 'expression_timestamp':'''@startuml
!$instant = 946684800
!$next = %date("yyyy-MM-dd", $instant + 86400)
!theme minty
class "$next" as TopazClock
@enduml
'''
}}
for mechanism, entries in cases.items():
 for name,source in entries.items():
  p=root/mechanism/(name+'.puml');p.parent.mkdir(parents=True,exist_ok=True);p.write_text(source)
