# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Maker inputs; generated independently of preserved labels and SVG geometry."""
from pathlib import Path
import subprocess, os, json
root=Path('/tmp/rustuml-cff2/fresh');root.mkdir(exist_ok=True)
cases={
'quote_modifiers':'cloud "Annex as [Input]" as Sector {\n+static class "Ledger Unit" as Unit {\n+value: int\n}\nrecord Snapshot\nUnit --> Snapshot\n}',
'boundary_renamed':'allowmixing\nnode "Transit" as Hub {\nclass Ticket\nBOUNDARY "Ticket desk" as Desk\nTicket -- Desk\n}',
'component_stereo':'cloud "Collector" as Net {\nclass Event\nmix_component "Receiver" as Rx <<Port>>\nEvent --> Rx\n}',
'component_reverse':'allowmixing\ncomponent "Egress" as Out\nclass Batch\nOut <-- Batch',
'package_slots':'package "Core Yard" <<Alpha>> as Ignored $one <<Beta>> <<Gamma>> [[https://example.org/core{Core docs}]] #Wheat {\nclass Sample\n}',
'queue_alias':'queue Shelf as "Holding shelf" <<Queued>> #Azure {\nclass Parcel\nclass Manifest\nParcel --> Manifest\n}',
'action_process':'action "Inspect" as A {\nclass Reading\n}\nprocess "Decide" as B {\nclass Result\n}',
'group_link':'package "Origin" as O {\nclass Start\n}\npackage "Destination" as D {\nclass Finish\n}\nO )--( D',
'description_route':'component "Bus" as B\nINTERFACE "Wire API" as I\nqueue Q\nB --> I\nI --> Q',
'package_link':'package "Link Yard" as Yard $one <<Public>> $two [[https://example.org/yard{Yard manual}]] {\nclass Gate\n}',
'cloud_nested':'cloud "Weather Grid" as Outer {\ncloud "Sector C" as Inner {\ninterface Hygrometer\n}\nclass Observer\n}',
'cloud_long':'cloud "Isobar region" {\nclass AtmosphericPressureRegister\nclass X\nAtmosphericPressureRegister --> X\n}',
'notes_cross_scope':'node "Sensor Hub" as Hub {\nclass Pressure\nnote right of Pressure : Local\n}\nnote left of Hub.Pressure : External',
'notes_nested':'frame "Field Station" as Outer {\nnode "Cabinet" as Inner {\nclass Meter\nnote bottom of Meter\ncomponent text\n[end of note]\nend note\n}\n}',
'floating_note':'node Yard {\nclass Gate\nnote "Manual" as N\nGate .. N\n}',
'empty_link':'package "Manual" as M [[https://example.org/manual{Read manual}]] {\n}',
}
for alignment in ['left','center','right']:
 cases['chrome_'+alignment]=f'{alignment} header\ncomponent page text\nshort\nendheader\ntitle\nInstrument register\nSummary\nendtitle\nnode \"Control Bay\" {{\nclass Gauge\n}}\n{alignment} footer\nFooter details\ntiny\nendfooter'
for decor in [')--','--(',')--(']:
 cases['endpoint_'+str(len(cases))]=f'class Mercury\nclass Cobalt\nMercury {decor} Cobalt'
for name,body in cases.items():(root/(name+'.puml')).write_text('@startuml\n'+body+'\n@enduml\n')
env=os.environ.copy();env['JAVA_HOME']='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home';env['PATH']=env['JAVA_HOME']+'/bin:'+env['PATH']
args=[env['JAVA_HOME']+'/bin/java','-jar','/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar','-tsvg',str(root)]
r=subprocess.run(args,env=env,capture_output=True,text=True)
(root/'java.log').write_text(r.stdout+r.stderr)
(root/'generation.json').write_text(json.dumps({'argv':args,'exit':r.returncode,'sources':sorted(cases)},indent=2)+'\n')
print(len(cases),r.returncode)
