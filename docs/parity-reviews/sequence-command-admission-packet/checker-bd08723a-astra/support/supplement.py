# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent/'supplement';C.mkdir(exist_ok=False);D=C/'reference-inputs';D.mkdir();cases=[]
sha=lambda b:hashlib.sha256(b).hexdigest()
def add(name,body,mechanism='sequence-registered-command-rejection',axes=None,valid=True,wrap=True,index=None):
    name='bd087_s_'+name
    src=('@startuml\n'+('participant Alder\nparticipant Osprey\nAlder -> Osprey: audit\n' if wrap else '')+body+'\n@enduml\n').encode();p=D/(name+'.puml');p.write_bytes(src)
    cases.append(dict(name=name,source=str(p),source_sha256=sha(src),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes or [name,'source-guided alternative form'],intended_valid=valid,mechanism=mechanism,registration_index=index))
for name,body,i in [('sprite_inline','sprite $junco 0123',23),('sprite_inline_colour','sprite $junco [1x1/color] F00',23),('sprite_internal','sprite $junco jar:archimate/activity',28),('sprite_internal_other','sprite $junco jar:archimate/junction',28),('style_internal','<style file=plantuml.skin>',30),('style_internal_quoted',"<style file='rose.skin'>",30),('sprite_lib','sprite $junco #cloudinsight#redis',25),('sprite_lib_svg','sprite $junco :cloudinsight:redis',26)]:add(name,body,index=i)
for tag,body in {
 'participant_dot':'participant Cedar.Branch\nCedar.Branch -> Alder: dotted',
 'participant_at':'participant Cedar@Branch\nCedar@Branch -> Alder: at sign',
 'participant_negative_order':'participant Cedar order -3\nCedar -> Alder: ordered',
 'participant_compact_stereo':'participant Cedar<<stereotype>>\nCedar -> Alder: stereo',
 'participant_compact_colour':'participant Cedar#Wheat\nCedar -> Alder: paint',
 'title_colon':'title: Audit',
 'title_space_colon':'title : Audit',
 'arrow_alias':'"Cedar Branch" as Cedar -> Alder: renamed',
 'arrow_extra_receivers':'Alder -> Osprey & Cedar: audience',
 'arrow_style_plain':'Alder -[plain]-> Osprey: style',
 'arrow_style_norank':'Alder -[norank]-> Osprey: style',
 'skinparam_nested_order':'skinparam sequence {\nparticipant {\nBackgroundColor Wheat\n}\nArrowColor Olive\n}\nAlder -> Osprey: color',
 'legend_embedded_title':'legend\ntitle Audit\nend legend',
 'legend_embedded_legend':'legend\nlegend Audit\nend legend',
 'note_terminator':'rnote over Alder\nopaque\nend hnote',
 'ref_quote_lines':'ref over Alder\n\' not a comment?\nend ref',
 'pragma_whitespace':'!pragma teoz\ttrue',
 'skinparam_next_line_brace':'skinparam sequence\n{\nArrowColor Olive\n}',
}.items():add(tag,body)
for shape,body in {'participant':'participant Cedar #Wheat-Olive','note':'note over Alder #Wheat-Olive: walnut','box':'box "Cedar" #Wheat-Olive\nparticipant Cedar\nend box'}.items():
    for separator in ['-','/','|','\\']:
        add('gradient_'+shape+'_'+str(ord(separator)),body.replace('Wheat-Olive','Wheat'+separator+'Olive'),'sequence-command-color-execution',[shape,'gradient separator '+separator,'warning observability'])
for action in ['footbox','footboxon','footboxoff','hidefootbox','showfootbox']:
    add('bare_'+action,action+'\nAlder -> Alder: recursion','sequence-footbox-command-consumption',[action,'self and internal topology'])
for scalar in ['\\n','\\t','&#10;','é🦉',"'&<>"]:
    for site in ['quoted_identifier','quoted_url','tooltip']:
        value='Fern'+scalar+'House'
        body={'quoted_identifier':f'participant "{value}"\n"{value}" -> Alder: metadata','quoted_url':f'participant Cedar [["https://example.org/{value}"]]','tooltip':f'participant Cedar [[https://example.org/{{{value}}}]]'}[site]
        add('attribute_'+site+'_'+str(len(cases)),body,'sequence-local-attribute-serialization',[site,'escaped content '+scalar])
for units in [2000,2001]:
    for astral in [False,True]:
        prefix='Osprey --> Alder: ';room=units-len(prefix);label='🦉'*(room//2)+'x'*(room%2) if astral else 'v'*room
        add('size_arrow_only_'+str(units)+'_'+str(astral),prefix+label,'sequence-command-size-dispatch-correction',[str(units)+' UTF16','arrow-only factory fallthrough','astral' if astral else 'ASCII'],wrap=False)
manifest=dict(schema_version=1,purpose='Source-guided supplement, immutable first batch retained',cases=cases);raw=(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw)
bundle=dict(manifest=manifest,manifest_sha256=sha(raw),files=[dict(path=r['name']+'.puml',sha256=r['source_sha256'],base64=base64.b64encode(Path(r['source']).read_bytes()).decode()) for r in cases],resources=[]);data=gzip.compress(json.dumps(bundle,ensure_ascii=False).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(data)
print(json.dumps(dict(cases=len(cases),manifest_sha256=sha(raw),bundle_sha256=sha(data))))
