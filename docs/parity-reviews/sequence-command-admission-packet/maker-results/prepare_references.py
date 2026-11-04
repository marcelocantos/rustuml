# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent;sha=lambda b:hashlib.sha256(b).hexdigest()
controls=[
('unknown_command','N -> S: seed\nfling N sideways\n',False,['unregistered command','identifier topology']),
('invalid_suffix','N -> S: seed\nhide footbox garbage\n',False,['registered prefix','trailing token']),
('invalid_gap','participant N\nparticipant S\nN\u2003->\u2003S: seed\n',False,['Pattern2 separator','unconsumed message']),
('known_short','N -> S: seed\n?-> N: short\n',True,['recognized unimplemented command','external']),
('known_quoted_activation','participant "North Pier"\n"North Pier" -> S: seed\nactivate "North Pier"\nS -> "North Pier": response\ndeactivate "North Pier"\n',True,['recognized unimplemented command','quoted activation']),
('known_participant_block','participant N [\nNorthern\nPier\n]\nN -> S: seed\n',True,['recognized unimplemented multiline','participant']),
('color_simple_invalid','N -> S: seed\nnote over N #CeruleanTypo: bad\n',False,['execution error','unknown simple color']),
('color_composite_terminator','N -> S: seed\nrnote over N #back:LightBlue: cargo\n',False,['color alternative backtracking','colon termination']),
('color_composite_valid','N -> S: seed\nhnote over N #back:LightBlue : cargo\n',True,['composite color','space termination']),
('color_gradient','N -> S: seed\nnote over N #LightBlue/Yellow: cargo\n',True,['gradient color','simple color parser']),
('footbox_vt','N -> S: seed\nhide\vfootbox\n',True,['visibility command','VT optional gap']),
('footbox_compact','N -> S: seed\nhidefootbox\nshowfootbox\n',True,['visibility command','zero gap']),
('attribute_unicode','participant "North" as Árbol [[https://example.test/枝?q=é&x=雪]]\nÁrbol -> South: seed\n',True,['data attribute Unicode','hyperlink non-ASCII and ampersand']),
('attribute_nonbmp','participant "North" as N [[https://example.test/😀?q=𐐀]]\nN -> South: seed\n',True,['non-BMP scalar serialization','hyperlink'])]
p=C/'reference-inputs';p.mkdir();cases=[]
for name,body,valid,axes in controls:
 for teoz in [False,True]:
  n=name+('_teoz' if teoz else '_legacy');s=p/(n+'.puml');s.write_text('@startuml\n'+('!pragma teoz true\n' if teoz else '')+body+'@enduml\n');cases.append(dict(name=n,source=str(s),source_sha256=sha(s.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',env={},invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=axes+['teoz' if teoz else 'legacy'],intended_valid=valid))
m=dict(schema_version=1,purpose='Registered-command rejection, explicit unsupported-command residue, color execution, footbox and local SVG attribute serialization',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw);bundle=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=s.name,sha256=sha(s.read_bytes()),base64=base64.b64encode(s.read_bytes()).decode()) for s in sorted(p.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z);print(len(cases),'manifest',sha(raw),'gzip',sha(z))
