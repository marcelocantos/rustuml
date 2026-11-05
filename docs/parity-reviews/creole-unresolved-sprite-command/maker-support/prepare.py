# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64, hashlib, json
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-maker'); R=Path.cwd(); N=S/'new';N.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
sprite='sprite $gem [2x2/16] {\nF0\n0F\n}\n'
cases=[
('missing_lead','class "<$absent> Cedar" as A',['missing name','leading command']),
('missing_mid','class "Cedar<$absent>Grove" as A',['missing name','adjacent runs']),
('missing_runs','class "Cedar <$absent> Grove <$other> end" as A',['multiple names','surrounding spaces']),
('missing_styled','class "**Cedar <$absent> Grove** end" as A',['bold','nested command']),
('missing_size','class "<size:20>Cedar <$absent> Grove</size> end" as A',['size','command boundary']),
('missing_lines','class "Cedar\\n<$absent>\\nGrove" as A',['multiple lines','empty stripe']),
('missing_note','class Anchor\nnote right\nCedar<$absent>Grove\nend note',['note consumer','adjacent runs']),
('missing_seq','Alice -> Bob : Cedar<$absent>Grove',['sequence consumer','adjacent runs']),
('empty_seq','Alice -> Bob : <$absent>',['sequence consumer','empty stripe']),
('empty_styled','Alice -> Bob : <size:20><$absent></size>',['empty stripe','closed size']),
('empty_open_style','Alice -> Bob : <size:20><$absent>',['empty stripe','open size']),
('grammar_valid','class "a <$oak/leaf-2> b <#red$ghost{scale=2,color=blue}> c <$ghost,color:red> end" as A',['name grammar','scale color syntax']),
('grammar_invalid','class "a <$bad.name> b <$bad name> c <$> d <$ghost*bad> end" as A',['invalid tokens','literal text']),
('escape_url','class "~<$ghost> / <space key> / http://example.test/a / [[https://example.test/ path]] end" as A',['escaping and URL','plain angle text']),
('defined_note',sprite+'class Anchor\nnote right\n<$gem> Cedar\nend note',['defined sprite','note consumer']),
('defined_scale_color',sprite+'class Anchor\nnote right\n<#red$gem{scale=2}> Cedar <$gem{color=blue}> end\nend note',['defined sprite','scale color']),
('defined_class',sprite+'class "<$gem> Cedar" as A',['defined sprite','class consumer']),
('invalid_source','class "unterminated',['syntax negative','source validity']),
]
rows=[]
for name,body,axes in cases:
 p=N/(name+'.puml');assert not p.exists();p.write_text('@startuml\n'+body+'\n@enduml\n')
 rows.append(dict(source=str(p),mechanism='creole-unresolved-sprite-command',axes=axes,negative=name=='invalid_source',intended_validity='intended_syntax_negative' if name=='invalid_source' else 'intended_valid',cwd='new',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
manifest=S/'new-manifest.json';dump(manifest,rows)
probe=R/'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java'
resources=[dict(local_path=str(p),remote_relative_path=('new/' if p.parent==N else '')+p.name,sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(N.iterdir())+[probe]]
old=json.loads((R/'docs/parity-reviews/tim-memory-trie-text-lookup/checker8-support/remote-request.json').read_bytes());contracts=[]
for row in rows:
 p=Path(row['source']);c=dict(old['case_contracts'][0]);c.update(source=str(p),source_sha256=sha(p.read_bytes()));c['probe_argv']=c['probe_argv'][:-2]+['<worker>/new/'+p.name,'<worker>/new'];contracts.append(c)
request=dict(old,manifest_path=str(manifest),manifest_sha256=sha(manifest.read_bytes()),case_count=len(rows),resources=resources,case_contracts=contracts)
p=S/'remote-request.json';dump(p,request)
dump(S/'input-bundle.json',dict(manifest=rows,request=request,files=[{**r,'base64':base64.b64encode(Path(r['local_path']).read_bytes()).decode()} for r in resources]))
for p in [manifest,p,S/'input-bundle.json']:print(str(p),sha(p.read_bytes()))
