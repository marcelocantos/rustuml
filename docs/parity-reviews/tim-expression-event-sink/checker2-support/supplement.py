# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import importlib.util,json
from pathlib import Path
spec=importlib.util.spec_from_file_location('checker','/tmp/rustuml-tim-check-2fa74908/checker.py');c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
rows=[]
def add(m,n,raw,axes,**kw):
 p=Path('test-diagrams/perturbations')/m/('checker2_'+n+'.puml');c.save(p,raw.encode());rows.append(dict(source=str(p),mechanism=m,kind='fresh supplemental',axes=axes,**kw))
e='tim-evaluated-event-stream';x='tim-parser-error-propagation'
for n,raw in [('raw_tight','@startuml\nclass Ash\nclass Elm\nAsh --> Elm\n@enduml\n'),('raw_blank_crlf','@startuml\r\n\r\n\r\nclass Ash\r\n\r\nclass Elm\r\nAsh --> Elm\r\n\r\n@enduml\r\n'),('raw_named_trailing','@startuml canopy\n\nclass Ash\n\nclass Elm\nAsh --> Elm\n\n\n@enduml\n')]:add(e,n,raw,['leading/internal/trailing whitespace','CRLF or delimiter name'])
for name in ['act_skinparam2_shadowing_true','act_skinparam_shadowing','edge_whitespace_multiple_blank_lines']:
 ps=list(Path('test-diagrams/golden').rglob(name+'.puml'));assert len(ps)==1
 add(e,'raw_counterpart_'+name,ps[0].read_text(),['frozen historical GET input','additional raw-pipe reference'],original_source=str(ps[0]),reference_contract='additional raw-pipe counterpart; frozen GET pair unchanged')
add('tim-declaration-default-values','literal_default_type','''@startuml
!procedure $emit($a = "007")
!$sum = $a + 2
class "$sum" as Typed
!endprocedure
$emit()
@enduml
''',['quoted literal default','numeric-looking string type'])
inc=Path('test-diagrams/perturbations')/x/'checker2_sub_body.inc'
c.save(inc,b'!$bad = %date("QQ", 0)\n!startsub useful\nclass Selected\n!endsub\n')
add(x,'sub_ignored_prelude',f'@startuml\n!includesub {inc}!useful\nclass After\n@enduml\n',['external subblock extraction','fatal unselected prelude'])
inc2=inc.with_name('checker2_sub_error.inc');c.save(inc2,b'!startsub bad\n!$bad = %date("QQ",0)\n!endsub\n')
add(x,'sub_selected_error',f'@startuml\nclass Before\n!includesub {inc2}!bad\n@enduml\n',['external selected subblock','fatal pattern'],negative=True)
add(x,'inactive_plain_error','@startuml\n!if 0\n!$v = %date("QQ",0)\n!endif\nclass Fine\n@enduml\n',['inactive branch','invalid assignment'])
add(x,'signed_underflow','@startuml\nclass Before\n!$v = %date("yyyy",-2147483649)\n@enduml\n',['signed lower overflow','partial class output'],negative=True)
add('tim-expression-event-sink','theme_from_order','''@startuml
!$count = 0
!function $name()
!$count = $count + 1
!return "custom"
!endfunction
!function $directory()
!$count = $count * 10
!return "test-diagrams/perturbations/tim-expression-event-sink/checker2-themes"
!endfunction
!theme $name() from $directory()
class "$count" as Order
@enduml
''',['theme selector versus from evaluation','side effect ordering'])
c.save(Path('test-diagrams/perturbations/tim-expression-event-sink/checker2-themes/puml-theme-custom.puml'),b'skinparam BackgroundColor #abcdee\n')
# The theme is a helper input, not a standalone diagram: store the helper with .inc
# and materialize the required theme filename only while commands run.
(c.SUP/'supplement-manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
print(len(rows))
