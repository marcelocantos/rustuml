# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Fresh checker inputs, from pinned command grammars; no Java execution."""
import base64, gzip, hashlib, json, re
from pathlib import Path

C = Path(__file__).parent
P = Path('/Users/marcelo/.t3/worktrees/rustuml/t3-8eddc9af/docs/parity-reviews/sequence-command-admission-packet/maker-results')
inv = json.loads((P/'inventory.json').read_text())
out = C/'reference-inputs'; out.mkdir(exist_ok=False)
sha = lambda b: hashlib.sha256(b).hexdigest()
cases = []
def add(name, body, axes, valid=True, mechanism='sequence-registered-command-rejection', index=None, prefix=''):
    name = 'bd087_'+name
    source = ('@startuml\n'+prefix+'participant Juniper\nparticipant Kestrel\nJuniper -> Kestrel: checkpoint\n'+body+'\nKestrel --> Juniper: receipt\n@enduml\n').encode()
    path=out/(name+'.puml');path.write_bytes(source)
    cases.append(dict(name=name,source=str(path),source_sha256=sha(source),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes,intended_valid=valid,mechanism=mechanism,registration_index=index))

# Each position corresponds to the independently read 81-command registration order.
forms = [
'title: Walnut observatory', 'mainframe: Walnut observatory', 'caption: Walnut observatory',
'caption\nWalnut\nobservatory\nend caption','title\nWalnut\nobservatory\nend title',
'legend top right\nWalnut\nobservatory\nend legend','legend: Walnut observatory',
'left footer: Walnut observatory','left footer\nWalnut\nobservatory\nend footer',
'right header: Walnut observatory','right header\nWalnut\nobservatory\nend header',
'set separator ::','', '!pragma teoz true','!assume transparent dark',
'skinparam SequenceArrowColor Olive', 'skinparam sequence {\ue100ArrowColor Olive\ue100}',
'skinparam sequence {\nArrowColor Olive\nParticipantBackgroundColor Wheat\n}',
'skin rose','minwidth 170','page 2 x 1','rotate',
'sprite $walnut [2x2/16] {\n01\n23\n}', 'sprite $walnut [1x1/16z] 0',
'sprite $walnut <svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"><rect width="2" height="2"/></svg>',
'sprite $walnut #archimate#application', 'sprite $walnut :archimate:application',
'sprite $walnut <svg xmlns="http://www.w3.org/2000/svg" width="2" height="2">\n<rect width="2" height="2"/>\n</svg>',
'sprite $walnut walnut_missing.png','<style>\nsequenceDiagram {\nArrowColor Olive\n}\n</style>',
'<style file=walnut_missing.css>', 'scale 1/2','scale 270*180','scale 270 width',
'scale max 270 width','scale max 180 height','scale max 270*180',
'hide empty description','hide public methods','hide stereotype','hide unlinked',
'activate Juniper #Wheat\ndeactivate Juniper','activate Juniper\ndeactivate',
'participant "Walnut node" as Walnut #Wheat','participant Walnut as "Walnut node" #Wheat',
'participant Walnut as Hazel #Wheat','participant "Walnut node" #Wheat',
'participant Walnut [\nWalnut node\n]', 'Juniper -> Kestrel: additional',
'[-> Kestrel: inbound','Juniper ->]: outbound',
'note left of Juniper: walnut','note over Juniper,Kestrel: walnut','note across: walnut',
'box "Walnut annex" #Wheat\nparticipant Walnut\nend box',
'box "Walnut annex"\nparticipant Walnut\nend box',
'alt walnut\nJuniper -> Kestrel: branch\nelse hazel\nKestrel -> Juniper: reverse\nend',
'Juniper ++ #Wheat\nJuniper --','activate Kestrel\nreturn walnut',
'note right: walnut','note over Juniper\nwalnut\nend note',
'note over Juniper,Kestrel\nwalnut\nend note','note bottom\nwalnut\nend note',
'note across\nwalnut\nend note','newpage: Walnut','ignore newpage','autonewpage 300',
'== Walnut ==','||17||','ref over Juniper,Kestrel: Walnut',
'ref over Juniper,Kestrel\nWalnut\nend ref','autonumber 6 2 "<b>00</b>"',
'autonumber\nautonumber stop','autonumber 6\nautonumber stop\nautonumber resume 3',
'autonumber 3.2\nautonumber inc A','autoactivate on','hide footbox','... Walnut ...',
'footbox off','url of Juniper is [[https://example.org/walnut]]',
'{first} Juniper -> Kestrel: first\n{second} Kestrel -> Juniper: second\n{first} <-> {second}: span']
assert len(forms)==81
for i,body in enumerate(forms):
    # Explicit execution resources are absent by design: syntax-valid resource
    # commands may fail execution; intended validity is retained, never relabelled.
    add(f'reg_{i:02d}_plain',body,[inv[i]['class'],'canonical form','two-way sequence context'],index=i)
    unusual=body.replace('Walnut','Cypress').replace('walnut','cypress')
    unusual=re.sub(r'^[A-Za-z]+',lambda m:m[0].upper(),unusual,count=1)
    unusual=unusual.replace(' ','\u00a0',1)
    add(f'reg_{i:02d}_unusual',unusual,[inv[i]['class'],'ASCII uppercase first keyword and first NBSP','self-message context'],index=i,prefix='Juniper -> Juniper: calibration\n')
    # Corrupt only the tested line, retaining a complete valid sequence around it.
    lines=body.split('\n');lines[0]='@'+lines[0]+'@@'
    add(f'reg_{i:02d}_near_miss','\n'.join(lines),[inv[i]['class'],'illegal prefix and suffix','valid sequence surrounding'],False,index=i)

for kind,begin,end in [('group','group willow','end'),('alt','alt willow','end'),('loop','loop 2','end'),('box','box "Willow"','end box'),('ref','ref over Juniper','end ref'),('note','note over Kestrel','end note'),('legend','legend left','end legend'),('title','title','end title'),('header','header','end header'),('footer','footer','end footer')]:
    for payload in ['nonsense walnut token','Juniper -> Kestrel: inside']:
        add(f'body_{kind}_{"command" if "->" in payload else "unknown"}',begin+'\n'+payload+'\n'+end,[kind,'body command versus text'],kind in ['ref','note','legend','title','header','footer'] or '->' in payload)
for tag,body in {
 'macro_valid':'!procedure $emit()\nhidefootbox\n!endprocedure\n$emit()',
 'macro_invalid':'!procedure $emit()\nwalnut impossible command\n!endprocedure\n$emit()',
 'comment_blank':"' walnut comment\n\n/' multiline\nwalnut nonsense\n'/\n\t\n\u00a0",
 'skinparam_nested':'skinparam sequence {\nparticipant {\nBackgroundColor Wheat\n}\nArrowColor Olive\n}',
 'skinparam_invalid':'skinparam sequence {\nwalnut\n}',
 'style_body':'<style>\nsequenceDiagram {\nparticipant {\nBackgroundColor Wheat\n}\n}\n</style>',
 'legend_spaced_end':'legend left\nwalnut\nend\tlegend',
 'ref_bare_end':'ref over Juniper\nwalnut\nend',
 'note_mixed_close':'hnote over Juniper\nwalnut\nend rnote',
}.items():add(tag,body,[tag,'block and preprocessing ownership'],'invalid' not in tag)

colour_templates={
 'participant':'participant Walnut #{c}', 'arrow':'Juniper -[#{c}]-> Kestrel: paint',
 'exo_left':'[-> Kestrel ++ #{c}: paint','exo_right':'Juniper ->] ++ #{c}: paint',
 'note':'note over Kestrel #{c}: paint','rnote':'rnote left #{c}: paint',
 'note_multi':'note over Juniper,Kestrel #{c}\npaint\nend note',
 'group':'group#{c} Walnut\nJuniper -> Kestrel: paint\nend',
 'group_secondary':'alt #Wheat #{c} Walnut\nJuniper -> Kestrel: paint\nend',
 'box':'box "Walnut" #{c}\nparticipant Walnut\nend box',
 'divider':'== <color:{c}>Walnut</color> ==',
 'ref':'ref#{c} over Juniper: paint',
 'ref_multi':'ref#{c} over Juniper\npaint\nend ref',
 'ref_ignored_back':'ref over Juniper #{c}\npaint\nend ref',
 'activation':'activate Juniper #{c}\ndeactivate Juniper',
 'activation_second':'activate Juniper #Wheat #{c}\ndeactivate Juniper',
 'activation_short':'Juniper ++ #{c}\nJuniper --',
 'return':'activate Kestrel\nreturn #{c} paint',
}
for tag,template in colour_templates.items():
    for c in ['NotAColourAtAll','automatic','background','transparent','aBc','11223344']:
        add('colour_'+tag+'_'+c,template.format(c=c),[tag,'color '+c,'post-selection execution'],c!='NotAColourAtAll', 'sequence-command-color-execution')
for c in ['Wheat/Olive','Wheat-NotAColourAtAll','transparent|Olive','Olive\\automatic','back:Wheat;line:Olive','back:NotAColourAtAll;line.dashed','line.dotted','shadowing:true','shadowing:NotAColourAtAll','text:application','Wheat/abc','abc|11223344']:
    for tag in ['participant','note','box']:
        add('gradient_'+tag+'_'+str(len(cases)),colour_templates[tag].format(c=c),[tag,'composite or gradient '+c],not ('NotAColourAtAll' in c and not c.startswith('shadowing:')) and not ('transparent|' in c or 'automatic' in c),'sequence-command-color-execution')

for name,space in [('zero',''),('sp',' '),('tab','\t'),('vt','\v'),('ff','\f'),('cr','\r'),('lf','\n'),('nbsp','\u00a0'),('em','\u2003'),('nnbsp','\u202f'),('nel','\u0085'),('ls','\u2028'),('ps','\u2029')]:
    for action in ['hide','show']:
        for engine in ['legacy','teoz']:
            body='hide footbox\n'+action+space+'footbox\nJuniper -> Juniper: own tail'
            add(f'footbox_{action}_{name}_{engine}',body,[name,action,'self-arrow and preceding hide',engine],name not in ['cr','lf','em','nnbsp','nel','ls','ps'],'sequence-footbox-command-consumption',prefix='!pragma teoz true\n' if engine=='teoz' else '')

for name,char in [('accent','é'),('nbsp','\u00a0'),('astral','🦉'),('tab','\t'),('cr','\r'),('lf','\n'),('amp','&'),('lt','<'),('gt','>'),('quot','"'),('apos',"'")]:
    for site in ['tooltip','url','id']:
        text='Birch'+char+'Harbor'
        body={'tooltip':f'participant Walnut [[https://example.org/tool{{{text}}}]]',
              'url':f'participant Walnut [[https://example.org/{text}]]',
              'id':f'participant "{text}"\n"{text}" -> Juniper: identity'}[site]
        add(f'attribute_{site}_{name}',body,[site,name,'participant and message attributes'],name not in ['lf','cr'] and not (site=='id' and name=='quot'),'sequence-local-attribute-serialization')

# No copied size8 inputs: change arrow direction, alias names and supplementary scalar.
for units in [1999,2000,2001,2002]:
    for astral in [False,True]:
        prefix='Kestrel --> Juniper: '
        room=units-len(prefix);label=('🦉'*(room//2)+'x'*(room%2)) if astral else 'w'*room
        add(f'size_{units}_{"astral" if astral else "ascii"}',prefix+label,['UTF16 units '+str(units),'reverse dashed arrow','astral' if astral else 'ASCII'],'True'=='True','sequence-command-size-dispatch-correction')

manifest=dict(schema_version=1,purpose='Independent adversarial sequence admission checker bd08723a',cases=cases)
raw=(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw)
bundle=dict(manifest=manifest,manifest_sha256=sha(raw),files=[dict(path=r['name']+'.puml',sha256=r['source_sha256'],base64=base64.b64encode(Path(r['source']).read_bytes()).decode()) for r in cases],resources=[])
data=gzip.compress(json.dumps(bundle,ensure_ascii=False).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(data)
print(json.dumps(dict(cases=len(cases),manifest_sha256=sha(raw),bundle_sha256=sha(data)),indent=2))
