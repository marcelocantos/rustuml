# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,json,hashlib
c=pathlib.Path(__file__).parent; d=c/'sources'; d.mkdir(exist_ok=True); cases=[]
names=['sequence-note-display-normalization','sequence-effective-arrow-color','sequence-legacy-raw-life-variations','sequence-teoz-parallel-tile-ordinates','sequence-effective-note-activation-style','sequence-note-textblock-dimensions','sequence-note-shadow-value-materialization']
def add(m,name,body,axes):
 p=d/(name+'.puml');p.write_text('@startuml\n'+body+'@enduml\n');cases.append(dict(name=name,remote_name=name+'.puml',mechanism=names[m],source=str(p),source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),timezone='GMT+11:00',cwd='PRIVATE_SEQUENCE_CHECK_EF020AA1',resources=[],invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_SEQUENCE_CHECK_EF020AA1','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes))
def style(text):return '<style>\n'+text+'\n</style>\n'
for e in [0,1]:
 pr='!pragma teoz true\n' if e else ''
 add(0,f'display_inline_{e}',pr+'skinparam noteFontSize 12\nQuartz -> Fern : call\nnote right : short\\n**Wide words**\\tZ\\\\q\nFern --> Quartz : done\n',['inline newline/tab/backslash','engine/font/style'])
 add(0,f'display_block_{e}',pr+'skinparam noteFontSize 12\nQuartz -> Fern : call\nnote right\nshort\\n**Wide words**\\tZ\\\\q\nend note\nFern --> Quartz : done\n',['block literal escape','engine/font/style'])
 add(0,f'display_natural_{e}',pr+'Quartz -> Fern : call\nnote right : tiny\\rconsiderably longer\\lEnd\n',['natural right/left alignment','engine/unequal lines'])
 add(1,f'color_primitives_{e}',pr+'Tern -[#2468ac80]> Pip : filled\nTern -[#2468ac01]o> Pip : circle\nPip -[#2468acfe]>> Pip : open self\nTern -[#transparent]x Pip : cross\nTern -[#a]> Pip : gray\n',['alpha 01/80/fe/no-paint/grayscale','line/polygon/ellipse/path/self/cross'])
 add(1,f'color_return_{e}',pr+'participant Tern\n[-> Tern ++ : enter\nreturn #application exit\nTern -> Tern ++ : recurse\nreturn #2468ac00 invisible\nTern -> Tern ++ : again\nreturn #2468acff opaque\n',['return/external/self','registered/alpha zero/opaque'])
 add(4,f'style_siblings_{e}',pr+style('sequenceDiagram {\n lifeLine {\n LineColor red\n }\n activationBox {\n LineColor blue\n }\n note {\n HorizontalAlignment right\n Shadowing 2.25\n }\n}')+'skinparam defaultTextAlignment center\nskinparam noteTextAlignment left\nWagtail -> Thrush ++ : start\nnote right : i\\nwider instruction\nreturn end\n',['scoped sibling inheritance','note component/block alignment/shadow'])
 add(4,f'style_root_{e}',pr+style('root {\n LineColor purple\n}\nsequenceDiagram {\n lifeLine {\n LineColor orange\n }\n}')+'Wagtail -> Thrush ++ : start\nThrush -> Thrush ++ : nested\nreturn inner\nreturn end\n',['root vs sibling inheritance','engine/nesting'])
 add(5,f'text_precision_{e}',pr+'skinparam noteFontSize 11\nskinparam noteTextAlignment center\nWagtail -> Thrush : go\nnote right : iii\\nWWWWWW\\n<size:16>tall</size>\nThrush -> Wagtail : after\n',['fractional full line widths/mixed height','engine/alignment/multiline'])
 add(5,f'text_theme_self_{e}',pr+'!theme cerulean\nMagpie -> Finch ++ : begin\nFinch -> Finch ++ : recursive\nnote right : compact\\nlonger allocation\nreturn nested\nreturn finished\n',['theme preferred allocation','engine/self note/multiline'])
for i,tail in enumerate(['','deactivate Egret\n','deactivate Egret\ndeactivate Egret\n']):
 add(2,f'life_unmatched_{i}','participant Robin\nparticipant Egret\nRobin -> Robin : seed\ndeactivate Egret\ndeactivate Egret\nactivate Egret\nRobin -> Egret : work\nactivate Egret\nRobin -> Egret : nested\n'+tail+'Robin -> Robin : last\n',['two unmatched closes/nested opens','zero/one/two explicit closing events'])
add(2,'life_return_ownership','participant Egret\nparticipant Robin\nRobin -> Robin : seed\ndeactivate Egret\nRobin -> Egret ++ : own\nEgret -> Egret ++ : nested\nreturn inner\nreturn outer\n',['raw close versus command stack','reverse declarations/nested returns'])
for i,prior in enumerate(['Lark -> Lark ++ : solo\\nsecond\n','note over Lark : unrelated\\nsecond\n','Lark -> Dove ++ : ordinary\n']):
 add(3,f'teoz_contact_{i}','!pragma teoz true\nparticipant Lark\nparticipant Dove\n'+prior+'& Lark -> Dove ++ : join\n& Dove -> Dove ++ : nested\nDove --> Dove -- : close\nDove --> Lark -- : reply\ndestroy Dove\n',['self/standalone note/ordinary predecessor','parallel lifecycles/contact/paint order'])
add(3,'teoz_create_order','!pragma teoz true\nparticipant Lark\nLark -> Lark ++ : first\ncreate participant Dove\n& Lark -> Dove : born\nDove -> Dove ++ : own\nDove --> Dove -- : done\ndestroy Dove\n',['creation in parallel level','destruction/activation order'])
# ValueImpl extracts digits/dots, so signs/units/exponents are not float grammar;
# malformed dots and no numeric chars default to 1.5, zero remains zero.
for i,value in enumerate(['-0px','.5px','1e2','1..2','false','.']):
 add(6,f'shadow_boundary_{i}',('!pragma teoz true\n' if i%2 else '')+style('sequenceDiagram {\n note {\n Shadowing '+value+'\n }\n}')+'skinparam shadowing false\nIbis -> Swift : call\nnote right : test\\nheight\n',['digit/dot extraction '+value,'engine/scoped override/global false'])
add(6,'shadow_global_scope',style('root {\n Shadowing 4\n}\nsequenceDiagram {\n note {\n Shadowing nonnumeric\n }\n}')+'Ibis -> Swift : call\nnote right : inherited\\nthen overridden\n',['root numeric versus scoped default','multiline/global inheritance'])
p=c/'reference-manifest.json';p.write_text(json.dumps(dict(schema_version=1,purpose='Independent seven-mechanism source-guided challenge at efc2e922',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',remote_private_directory='allocate with existing runner',cases=cases),indent=2)+'\n');print(len(cases),p,hashlib.sha256(p.read_bytes()).hexdigest())
