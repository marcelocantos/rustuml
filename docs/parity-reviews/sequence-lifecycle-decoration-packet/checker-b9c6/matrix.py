# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json
from pathlib import Path
cache=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-b9c6cf04'); inputs=cache/'reference-inputs';inputs.mkdir(exist_ok=True)
cases=[]
def add(mech,name,body,axes,engine=0):
    name='checker_b9c6_'+name
    text='@startuml\n'+('!pragma teoz true\n' if engine else '')+body.strip()+'\n@enduml\n'
    p=inputs/(name+'.puml');p.write_text(text)
    cases.append(dict(name=name,mechanism='sequence-'+mech,source=str(p),remote_relative_name=name+'.puml',source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),timezone='GMT+11:00',cwd='PRIVATE_CWD',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],stdin='exact source bytes',resources=[],axes=axes+['engine='+str(engine)]))
for e in (0,1):
    add('command-event-identity',f'identity_chain_e{e}','participant Cedar\nparticipant Birch\nparticipant Maple\nCedar -> Birch ++ : first\n& Birch -> Maple ++ : second\n& Maple -> Maple ++ : third\ndestroy Maple\n& return #DarkSlateGrey inner\nreturn middle\nreturn outer',['parallel ancestry length three','destroy with nested return ownership'],e)
    add('command-event-identity',f'identity_exo_e{e}','participant Birch\n[-> Birch ++ : enter\n& return #123abc80 leave',['external return ownership','RGBA return color'],e)
    add('command-event-identity',f'identity_symbolic_e{e}','Cedar -> Birch ++ : enter\nreturn #transparent vanish',['activation close','symbolic valid return color'],e)
    add('command-event-identity',f'identity_gray_e{e}','Cedar -> Birch : enter\nreturn #a gray\nreturn #12ab34cd alpha',['fallback repeated return','one and eight digit valid colors'],e)
    add('command-event-identity',f'identity_other_e{e}','participant Cedar\nparticipant Birch\nCedar -> Birch ++ : enter\nBirch -> Birch : self\ndeactivate Cedar\nactivate Cedar\nreturn leave',['self event ignores other participant prohibition','command stack owns unrelated activation'],e)
    add('command-event-identity',f'identity_reopen_e{e}','Cedar -> Birch ++ : enter\nBirch -> Cedar : reply\ndestroy Birch\nnote over Birch : intervening\nactivate Birch',['negative same-message reactivation','note preserves owner'],e)
    add('command-event-identity',f'identity_badcolor_e{e}','Cedar -> Birch : enter\nreturn #red_foo reject',['negative unknown return color','internal fallback'],e)
# Teoz branches whose tile/history treatment is independently important.
for name,body,axes in [
 ('life_parallel_note','Cedar -> Birch ++ : first\nnote over Birch : separated\n& Birch -> Cedar : second\ndeactivate Birch\nCedar -> Birch : tail',['parallel history crosses standalone note','deactivate callback']),
 ('life_parallel_multiline','Cedar -> Birch ++ : first\n& Birch -> Birch ++ : one\\ntwo\\nthree\n& Birch -> Cedar -- : last\nreturn unwind',['parallel contact maximum','self multiline and nested close']),
 ('life_create_forward','participant Cedar\ncreate Birch\nCedar -> Birch ++ : first\\nsecond\\nthird\nBirch -> Birch ++ : deeper\nBirch -> Cedar : done\ndestroy Birch',['create arrow taller than head','destroy nested activation']),
 ('life_create_reverse','participant Birch\nparticipant Cedar\ncreate Birch\nCedar -> Birch ++ : new\nBirch -> Birch ++ : deeper\nreturn inner\nBirch -> Cedar : done\ndestroy Birch',['reverse creation direction','return before destroy']),
 ('life_parallel_create','participant Cedar\nparticipant Birch\nCedar -> Birch : first\ncreate Maple\n& Birch -> Maple ++ : born\nMaple -> Birch : done\ndestroy Maple',['CREATE in parallel suffix','creation and destruction attachment']),
 ('life_standalone_destroy','Cedar -> Birch ++ : first\ngroup scope\nBirch -> Birch ++ : inner\nend\ndestroy Birch\nCedar -> Birch : after',['standalone destroy after group END','depth two'])]:add('teoz-life-event-scheduling',name,body,axes,1)
for e in (0,1):
    for k,(arrow,life) in enumerate([('x--x',''),('<--x',' ++'),('x-->',' --'),('--x',' --')]):
        add('self-cross-decoration',f'cross_e{e}_k{k}',f'Cedar -> Birch ++ : begin\nBirch -> Birch ++ : nested\nBirch {arrow} Birch{life} : edge\nBirch -> Cedar : after',['cross dressing='+arrow,'depth two with lifecycle='+repr(life)],e)
for k,(prefix,event,tail) in enumerate([
 ('Cedar -> Birch ++ : begin\nBirch -> Birch ++ : nested','Birch -> Birch : self','destroy Birch'),
 ('Cedar -> Birch ++ : begin','Birch -> Birch : self','note right : attached\ndestroy Birch'),
 ('Cedar -> Birch ++ : begin','group frame\nBirch -> Cedar : reply\nend','destroy Birch\ndestroy Cedar'),
 ('participant Cedar\ncreate Birch\nCedar -> Birch ++ : born','Birch -> Birch : self','destroy Birch')]):
    add('legacy-destroy-component',f'destroy_k{k}',prefix+'\n'+event+'\n'+tail+'\nCedar -> Birch : after',['ownership self/note/group','activation depth or CREATE ordering'])
for k,(arrow,active,note,msg) in enumerate([
 ('Cedar -> Birch',' ++','right','An extraordinarily long ordinary message label constrains the arrow'),
 ('Birch -> Cedar',' --','right','reply'),
 ('Cedar -> Birch','','left','ordinary'),
 ('Birch -> Cedar','','right','An extraordinarily long reverse message forces preferred width')]):
    add('legacy-message-note-envelope',f'envelope_k{k}',f'participant Cedar\nparticipant Birch\nCedar -> Birch ++ : enter\n{arrow}{active} : {msg}\nnote {note} : narrow\nreturn after',['direction and activation transition','preferred arrow versus actual width / note side'])
for k,(style,arrow,active) in enumerate([
 ('skinparam noteTextAlignment center','Cedar -> Birch',' ++'),
 ('skinparam noteTextAlignment right\nskinparam shadowing true','Birch -> Cedar',' --'),
 ('skinparam noteTextAlignment left\nskinparam shadowing true','Cedar -> Birch',''),
 ('!theme cerulean','Birch -> Cedar',' --'),
 ('<style>\nsequenceDiagram {\n note {\n  HorizontalAlignment center\n  Shadowing 7\n }\n}\n</style>','Cedar -> Birch',' ++'),
 ('skinparam noteTextAlignment center\nskinparam padding 12\nskinparam shadowing true','Birch -> Cedar','')]):
    add('legacy-message-note-alignment',f'alignment_k{k}',style+f'\nparticipant Cedar\nparticipant Birch\nCedar -> Birch ++ : begin\n{arrow}{active} : body\nnote right : first line\\na much longer second line\nreturn after',['alignment via style/theme/skinparam','shadow and activation/direction'])
request=dict(schema_version=1,purpose='Independent b9c6 source-guided checker matrix; raw CLI policy; no local Java',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',cases=cases)
p=cache/'reference-request.json';p.write_text(json.dumps(request,indent=2)+'\n');p.chmod(0o444)
for p2 in inputs.iterdir():p2.chmod(0o444)
print(len(cases),str(p),hashlib.sha256(p.read_bytes()).hexdigest())
