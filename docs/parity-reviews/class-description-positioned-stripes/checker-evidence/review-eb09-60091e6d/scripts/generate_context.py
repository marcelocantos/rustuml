# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,base64,gzip
from pathlib import Path
S=Path(__file__).resolve().parent;cases=[]
inputs={
'url_current_group_self':'package Basin {\nclass Anchor\nurl Basin [[https://example.org/self]]\n}',
'url_root_class_ambiguous':'class Memo\npackage Basin {\nclass Memo\nurl Memo [[https://example.org/current]]\n}',
'url_root_class_note_ambiguous':'class Memo\npackage Basin {\nnote "local memo" as Memo\nurl Memo [[https://example.org/current-note]]\n}',
'url_rooted_existing':'class Memo\npackage Basin {\nclass Other\nurl .Memo [[https://example.org/rooted]]\n}',
'url_bad_separator':'package Basin {\nclass Anchor\n}\nurl Basin..Anchor [[https://example.org/double]]',
'url_namespace_none':'set namespaceSeparator none\npackage Basin {\nnote "memo" as Memo\n}\nclass Anchor\nurl Memo [[https://example.org/none]]',
}
for name,body in inputs.items():
 p=S/'fresh'/(name+'.puml');p.write_text('@startuml\n'+body+'\n@enduml\n');p.chmod(0o444);b=p.read_bytes();cases.append(dict(name=name,source=str(p),remote_name=p.name,source_sha256=hashlib.sha256(b).hexdigest(),bytes=len(b),source_base64=base64.b64encode(b).decode(),axes=['quark context/root/namespace','target owner and ambiguity'],mechanisms=['class-command-url-named-quark','class-command-url-builder-context']))
m=json.loads((S/'fresh-manifest.json').read_text());m['cases']=cases;m['review']='class-packet6-context-60091e6d';p=S/'context-manifest.json';p.write_text(json.dumps(m,indent=2)+'\n');p.chmod(0o444);b=gzip.compress(p.read_bytes(),mtime=0);(S/'context-manifest.json.gz').write_bytes(b);print(p,hashlib.sha256(p.read_bytes()).hexdigest(),len(p.read_bytes()));print('gzip',hashlib.sha256(b).hexdigest(),len(b))
