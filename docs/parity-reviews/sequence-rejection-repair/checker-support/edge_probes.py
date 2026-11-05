# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Additional probes derived from unrepresented Java event/style branches."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
manifest=Path(__file__).with_name('fresh-manifest.json')
rows=json.loads(manifest.read_text())
cases=[]
for side in ['left','right']:
 for reverse in [False,True]:
  arrow=('[-> M' if not reverse else 'M ->[') if side=='left' else ('M ->]' if not reverse else 'M <-]')
  body='participant A\nparticipant M\nparticipant Z\nA -> M ++ : enter\ngroup border\n'+arrow+' : boundary\nend\nM -> Z : exit\n'
  cases.append(('sequence-teoz-event-history',f'external_{side}_{int(reverse)}',body,['external border side','external direction','group and active participant']))
cases.extend([
 ('sequence-teoz-callback-lifecycle','standalone_destroy','participant A\nparticipant B\nA -> B ++ : enter\ngroup done\nB -> A : close\nend\ndestroy B\nA -> A : after',['standalone destruction after group owner','self arrow after destruction']),
 ('sequence-teoz-real-bound-capture','box_before_note','box Title extending initial bounds\nparticipant A\nend box\nparticipant "Wide worker" as B\ngroup bounds\nnote over A,B: x\nA -> B : a very wide later constraint from this message\nend',['englober margins before Real compile','span allocation versus later arrow expansion']),
 ('sequence-reverse-self-component','cross','participant A\nparticipant B\nA -> B ++ : enter\nB x-- B : reverse cross\nnote left: beside cross',['reverse cross arrowhead','active depth and attached note']),
 ('sequence-return-configuration','nested_after_destroy','A -> B ++ : enter\nB -> C ++ : nested\ndestroy C\nreturn #blue first\nreturn #red next',['destroy before return','multiple activating destinations and color'])
])
for mech,name,body,axes in cases:
 p=ROOT/'test-diagrams/perturbations'/mech/('checker_d382_'+name+'.puml')
 with p.open('x') as f: f.write('@startuml\n!pragma teoz true\n'+body.rstrip()+'\n@enduml\n')
 rows.append(dict(source=str(p.relative_to(ROOT)),golden=str(p.with_suffix('.svg').relative_to(ROOT)),cohort=mech,axes=axes))
manifest.write_text(json.dumps(rows,indent=2)+'\n')
print(len(rows),'fresh source entries')
