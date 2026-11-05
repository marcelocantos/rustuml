# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent compact source-guided probes. Never launches a reference process."""
import itertools
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[4]
rows=[]
def put(mechanism,name,body,axes,teoz=True):
    path=ROOT/'test-diagrams/perturbations'/mechanism/('checker_d382_'+name+'.puml')
    path.parent.mkdir(parents=True,exist_ok=True)
    source='@startuml\n'+('!pragma teoz true\n' if teoz else '')+body.rstrip('\n')+'\n@enduml\n'
    with path.open('x') as f: f.write(source)
    rows.append(dict(source=str(path.relative_to(ROOT)),golden=str(path.with_suffix('.svg').relative_to(ROOT)),cohort=mechanism,axes=axes))

for engine,depth in itertools.product([False,True],[1,3]):
    m='sequence-return-configuration'
    body='participant "Outpost" as O\nparticipant "Worker station" as W\nO -> W ++ : first\n'
    body+='W -> W ++ : descend\n'*(depth-1)
    body+='note right: return owner remains\nreturn #335577 response\n'
    body+='return #darkgreen unwind\n'*(depth-1)
    body+='return #orange fallback\nreturn inherited\n'
    put(m,f'e{int(engine)}_depth{depth}',body,['engine','activation recursion and sequential color inheritance'],engine)

for engine,reverse_order in itertools.product([False,True],[False,True]):
    decls=['participant "Long preceding head" as Q','participant "Small" as S','participant Tail']
    if reverse_order: decls.reverse()
    body='skinparam padding 2\n'+'\n'.join(decls)+'\nQ -> S ++ : enter\nS <<-- S ++ : reverse\n'
    body+='note left\nleft attached\nsecond line\nend note\nS <-- S -- : unwind\nnote right: opposite side\nS -> Tail : finish\n'
    put('sequence-reverse-self-component',f'e{int(engine)}_order{int(reverse_order)}',body,['engine','participant declaration order','reverse async head and attached note side'],engine)

for engine,shadow in itertools.product([False,True],[False,True]):
    body=f'skinparam shadowing {str(shadow).lower()}\nskinparam padding 4\nparticipant "Wide sentinel" as S\nparticipant P\nparticipant "Storage" as T\ngroup envelope\nnote over S,T: n\nS -> P : short\n...  longer wait ...\nP <- T : reverse\nend\n'
    put('sequence-preferred-component-flow',f'e{int(engine)}_shadow{int(shadow)}',body,['engine','shadow on spanning preferred dimensions','padding and delay composition'],engine)

for engine,later in itertools.product([False,True],[False,True]):
    body='title A title wider than the tiny participants\nheader Header\nlegend left\nlegend body\nendlegend\ncaption First line\\nSecond long line\nfooter Footer\nhide footbox\nparticipant I\nparticipant J\nI -> J ++ : enter\n'
    if later: body+='newpage\nJ <- J ++ : second page reversed recursion\nreturn back\n'
    body+='return done\n'
    put('sequence-page-wrapper-hierarchy',f'e{int(engine)}_page{int(later)}',body,['engine','page boundary with later lifecycle and self-arrow','wrapper order and multiline caption'],engine)

for order,creation in itertools.product([False,True],[False,True]):
    body=('participant B\nparticipant A\n' if order else 'participant A\nparticipant B\n')
    if creation: body+='create C\nA -> C ++ : born\nC -> B ++ : enter\n'
    else: body+='A -> B ++ : enter\n'
    body+='B -> B : active call\ndeactivate B\nactivate B #pink\nnote over A: separator\nB -> A : notify\ndestroy B\n'
    put('sequence-teoz-event-history',f'order{int(order)}_create{int(creation)}',body,['declaration order','creation/destruction and close/reopen ownership'])

for depth,separation in itertools.product([1,3],['group','note']):
    body='participant S\nparticipant T\nactivate S #lightblue\nS -> T ++ : enter\n'
    body+='T -> T ++ : recursion\n'*(depth-1)
    body+=('group bracket\nT -> S : inside\nend\n' if separation=='group' else 'note over T: ownership reset\n')
    body+='deactivate T\ndeactivate T\nT -> S : after closes\nactivate T #pink\nT -> T : reopened\ndeactivate T\n'
    put('sequence-teoz-callback-lifecycle',f'depth{depth}_{separation}',body,['activation depth','group END versus standalone note before repeated closes','initial activation'])

for order,nested in itertools.product([False,True],[False,True]):
    body='skinparam padding 2\n'+('participant "Very wide back office" as B\nparticipant A\n' if order else 'participant A\nparticipant "Very wide back office" as B\n')
    body+='group outer [ready]\n'
    if nested: body+='alt inner\n'
    body+='A <- A : reverse\nnote left: attached\nnote over B,A: x\n...  wait   ...\n'
    if nested: body+='else much longer alternate branch annotation\nB -> A : branch\nend\n'
    body+='end\nA -> B : follow\n'
    put('sequence-teoz-group-tile-composition',f'order{int(order)}_nested{int(nested)}',body,['declaration order versus explicit note endpoints','group nesting and else','attached reverse note and delay'])

for reverse,side in itertools.product([False,True],['left','right']):
    body='skinparam padding 3\nskinparam shadowing true\nparticipant "Edge" as E\nparticipant "Wider middle participant" as M\nparticipant Z\nE -> M ++ : enter\n'
    body+=f'M {"<--" if reverse else "-->"} M ++ : nested\nnote {side}\nattached words\nmore\nend note\nM -> Z : next\nnote over Z,E: span\n'
    put('sequence-teoz-note-component-flow',f'reverse{int(reverse)}_{side}',body,['self direction','attached note side','asymmetric span endpoint order and shadow'])

for order,wide in itertools.product([False,True],[False,True]):
    body='skinparam padding 1\nskinparam shadowing true\n'+('participant "Wider end" as B\nparticipant A\n' if order else 'participant A\nparticipant "Wider end" as B\n')
    body+='group sampled\nnote over A,B: x\nA -> B : '+('A constraint far wider than both participant heads combined' if wide else 'm')+'\nend\n'
    put('sequence-teoz-real-bound-capture',f'order{int(order)}_wide{int(wide)}',body,['asymmetric participant declaration order','constraint expansion after captured bounds'])

for engine,spacing in itertools.product([False,True],['   lag','lag    ']):
    body='skinparam padding 3\nparticipant A\nparticipant "Wider head" as B\ngroup delay window\nA -> B : before\n...'+spacing+'...\nB -> A : after\nend\n'
    put('sequence-delay-label-allocation',f'e{int(engine)}_{"leading" if spacing.startswith(" ") else "trailing"}',body,['engine','asymmetric leading versus trailing allocation whitespace','group and padding'],engine)

for engine,quoted in itertools.product([False,True],[False,True]):
    title='"Northern district  "' if quoted else 'Northern district  '
    body=f'box {title} #lightblue\nparticipant A\nparticipant B\nend box\nparticipant C\nA -> C : outside\nB -> A : inside\nfooter End\n'
    put('sequence-box-title-grammar',f'e{int(engine)}_quoted{int(quoted)}',body,['engine','quoted/unquoted trailing whitespace before color','multi-participant box'],engine)

for multi,nested in itertools.product([False,True],[False,True]):
    body='skinparam padding 2\nskinparam boxPadding 7\nbox An unusually wide outer title #lightblue\nparticipant A\n'
    if nested: body+='box Interior #pink\n'
    body+='participant B\n' if multi else ''
    if nested: body+='participant C\nend box\n'
    body+='end box\nparticipant Z\nA -> Z : outside\n'
    if multi: body+='B <- B : loop\n'
    body+='hide footbox\nfooter last band\n'
    put('sequence-teoz-englober-margins',f'multi{int(multi)}_nested{int(nested)}',body,['boxed participant count','nested englobers','boxPadding and hidden footbox'])

# Source-guided edge controls: distinct from the main compact matrices.
extras=[
 ('sequence-return-configuration','orphan_note','participant A\nnote over A: no owner\nreturn #purple orphan'),
 ('sequence-return-configuration','external_twice','participant A\n[-> A ++ : external\nreturn #pink first\nreturn second'),
 ('sequence-return-configuration','unknown_color','A -> B ++ : enter\nreturn #notacolor777 done'),
 ('sequence-return-configuration','parallel','A -> B ++ : enter\nB -> C ++ : nested\n& return #blue together\nreturn outer'),
 ('sequence-teoz-event-history','parallel','participant A\nparticipant B\nparticipant C\nA -> B : a\n& A -> C ++ : c\nB -> C : next\ndeactivate C'),
 ('sequence-teoz-event-history','create_then_destroy','participant A\ncreate B\nA -> B ++ : make\nB -> B : run\ndestroy B\nA -> A : done'),
 ('sequence-teoz-callback-lifecycle','delay_active','A -> B ++ : start\n... asleep ...\nB -> A : resume\ndeactivate B'),
 ('sequence-delay-label-allocation','multiline','A -> B : first\ngroup delay\n... one\\ntwo ...\nend\nB -> A : end'),
 ('sequence-page-wrapper-hierarchy','top_legend','title Short\nlegend top right\ntop legend\nendlegend\ncaption bottom\nA -> B : body'),
 ('sequence-page-wrapper-hierarchy','styled_caption','<style>\ncaption {\nFontSize 20\nMargin 6\nPadding 3\n}\n</style>\ncaption Wrapped\\nLarge\nA -> B : body'),
 ('sequence-preferred-component-flow','actor_shadow','skinparam shadowing true\nactor A\ndatabase B\ngroup kinds\nnote over A,B: span\nA -> B : body\nend'),
 ('sequence-teoz-group-tile-composition','hex_note','participant A\nparticipant B\ngroup shapes\nhnote over A,B: across\nA -> B : body\nend'),
]
for mechanism,name,body in extras: put(mechanism,name,body,['source-guided edge control',name])
(ROOT/'docs/parity-reviews/sequence-rejection-repair/checker-support/fresh-manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
print(len(rows),'fresh sources; references pending')
