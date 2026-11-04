# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,base64,gzip
from pathlib import Path
S=Path(__file__).resolve().parent
D=S/'fresh';D.mkdir(exist_ok=True)
cases=[]
def add(name,body,axes,mechanisms):
 text='@startuml\n'+body+'\n@enduml\n'
 p=D/(name+'.puml');p.write_text(text)
 cases.append(dict(name=name,source=str(p),remote_name=name+'.puml',source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),bytes=len(p.read_bytes()),source_base64=base64.b64encode(p.read_bytes()).decode(),axes=axes,mechanisms=mechanisms))
stripe='class-description-positioned-stripes';stereo='class-description-stereotype-font';margin='class-description-stereotype-margin-envelope';shadow='class-boundary-fashion-shadow';svek='class-description-svek-envelope';header='class-description-header-envelope-ownership';url='class-command-url-named-quark';builder='class-command-url-builder-context'
for symbol in ['actor','boundary']:
 for key,label in [('blankstyle',r'upper\n<size:31>\nlower'),('tabonly',r'\t\n\t\t'),('subsup',r'<size:7>t<sup>u</sup></size><size:28>W<sub>j</sub></size>\n<sup>A</sup><sub>z</sub>'),('mixedfamily',r'<font:monospaced>W\ti</font><size:9>j</size>\n<size:26><i>slanted</i></size>'),('spaceblank',r'  wide  \n   \nlast')]:
  add(symbol+'_'+key,f'{symbol} "{label}" as Orb\nclass Dock\nOrb -right-> Dock',['symbol kind',key,'linked LR'],[stripe,svek,header])
 for shape,label,size in [('wide','LONG_STEREOTYPE',8),('small','s',26)]:
  add(symbol+'_stereowrapper_'+shape,f'<style>\n{symbol} {{\n FontSize 17\n FontStyle bold\n stereotype {{\n FontSize {size}\n FontStyle plain\n FontColor DarkRed\n }}\n}}\n</style>\n{symbol} "<size:6>i</size>\\n<size:25>Wide label</size>" as Orb <<{label}>>\nclass Dock\nDock -down-> Orb',['stereotype wider/narrower','independent font style/size','reversed linked TB'],[stereo,margin,stripe,svek])
 for amount in ['0','0.5','13','-3']:
  add(symbol+'_shadow_'+amount.replace('.','p').replace('-','neg'),f'<style>\n{symbol} {{\n Shadowing {amount}\n LineThickness 3\n}}\n</style>\n{symbol} "m\\n<sub>n</sub>" as Orb\nclass Dock\nOrb --> Dock',['shadow zero/fraction/positive/negative','actor/boundary','stroke width'],[shadow,stripe,svek])
 for amount in ['0','11']:
  add(symbol+'_degenerate_'+amount,f'<style>\n{symbol} {{\n Shadowing {amount}\n}}\n</style>\n{symbol} "<size:25>top</size>\\n\\n<sub>end</sub>" as Orb <<small>>',['degenerated one entity','shadow','mixed/blank stripes'],[stripe,svek,margin,header])
# Native generic ownership contrast and a third description owner.
for kind in ['class','component','rectangle']:
 add(kind+'_generic_contrast',f'{kind} "<size:23>Header</size>" as Orb\nclass "Peer<T>" as Peer\nOrb --> Peer',['image owner','generic-looking markup','native peer'],[header,stripe])
# Tooltip alternatives, prefixing and endpoint owner all exercise CommandUrl beyond note renames.
for owner,decl,target in [('note','note "memo body" as Memo','Memo'),('class','class Anchor','Anchor'),('group','package Harbor {\nclass Anchor\n}','Harbor')]:
 add('url_'+owner+'_replace',f'{decl}\nurl {target} [[https://example.org/old{{Old}}]]\nurlfor {target} is [["doc with space"{{New}} caption]]\nskinparam topurl https://late.example/',['existing owner kind','replacement command','optional words/no gap'],[url,builder])
forms=[('tooltip','[[{outer {nested} tip}]]'),('tooltip_label','[[{Hint} visible label]]'),('quoted','[["a b?q=x&v=y"{hint} label]]'),('scheme_upper','[[HTTPS://example.org/a]]'),('file','[[file:/tmp/name]]'),('empty_tip','[[path{}]]'),('nested_tip','[[path{a {nested} b}]]')]
for name,form in forms:
 add('url_form_'+name,f'skinparam topurl https://prefix.example/\nclass Anchor\nnote "memo body" as Memo\nurl for Memo is {form}\nMemo .. Anchor',['UrlBuilder alternative',name,'topurl'],[url,builder])
add('url_unique_crosscontext','package First {\nnote "memo body" as Memo\n}\npackage Second {\nclass Anchor\nurl Memo [[https://example.org/unique]]\n}',['unique shortname','other current context'],[url,builder])
add('url_ambiguous_current','package First {\nnote "one" as Memo\n}\npackage Second {\nnote "two" as Memo\nurl Memo [[https://example.org/current]]\n}\nclass Anchor',['ambiguous shortname','current context'],[url,builder])
add('url_ambiguous_root','package First {\nnote "one" as Memo\n}\npackage Second {\nnote "two" as Memo\n}\nurl Memo [[https://example.org/root]]',['ambiguous shortname','absent current-context entity'],[url,builder])
add('url_forward','class Anchor\nurl Memo [[https://example.org/forward]]\nnote "later" as Memo',['command order','missing target'],[url,builder])
add('url_trailing','class Anchor\nurl Anchor [[https://example.org/]] trailing',['anchoring','existing class'],[url,builder])
add('url_inline_note','class Anchor\nnote "memo" as Memo [[https://example.org/]]',['declaration/command ownership','inline negative'],[url,builder])
add('url_empty','class Anchor\nurl Anchor [[]]',['strict empty URL','existing class'],[url,builder])
add('url_smart_quotes','class Anchor\nnote "memo" as Memo\nurl “Memo” [[“doc with spaces”{Hint}]]',['Java %g smart quotation','quoted code and URL'],[url,builder])
# Separate ownership from cluster geometry: same source with and without note URL.
for form in ['single','multi']:
 for linked in [False,True]:
  body='package Fleet {\nclass Vessel\n}\nclass Pier\n'
  body+=('note right of Fleet [[https://example.org/map]] : chart' if form=='single' else 'note right of Fleet [[https://example.org/map]]\nchart\nend note')
  if linked:body+='\nFleet --> Pier'
  add(f'group_{form}_{linked}',body,['note command form','group external linkage'],[url])
manifest=dict(schema_version=1,review='class-packet6-60091e6d',candidate='eb09cecbc90f5690c4a09bf49118f2feff8c1c12',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',timezone='GMT+11:00',argv=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],resources=[],cases=cases)
p=S/'fresh-manifest.json';p.write_text(json.dumps(manifest,indent=2)+'\n');p.chmod(0o444)
for c in cases:Path(c['source']).chmod(0o444)
b=gzip.compress(p.read_bytes(),mtime=0);(S/'fresh-manifest.json.gz').write_bytes(b)
print(len(cases),p,hashlib.sha256(p.read_bytes()).hexdigest(),len(p.read_bytes()));print('gzip',hashlib.sha256(b).hexdigest(),len(b))
