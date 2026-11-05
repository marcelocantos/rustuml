# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json
from pathlib import Path
c=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-b9c6cf04');request=json.loads((c/'reference-request.json').read_text());cases=[]
for case in request['cases']:
 if case['name'].startswith('checker_b9c6_alignment_k') and not case['name'].endswith('k5'):
  n=dict(case);n['name']+='_single';n['remote_relative_name']=n['name']+'.puml';n['source']=str(c/'reference-inputs'/n['remote_relative_name']);b=Path(case['source']).read_bytes().replace(b'first line\\na much longer second line',b'a much longer single line');Path(n['source']).write_bytes(b);n['source_sha256']=hashlib.sha256(b).hexdigest();n['axes']=case['axes']+['single-line control isolates escaped-note dependency'];cases.append(n)
for variant,note in [('escaped','note right : first line\\na much longer second line'),('block','note right\nfirst line\na much longer second line\nend note')]:
 n=dict(request['cases'][-1]);n['name']='checker_b9c6_envelope_'+variant;n['mechanism']='sequence-legacy-message-note-envelope';n['remote_relative_name']=n['name']+'.puml';n['source']=str(c/'reference-inputs'/n['remote_relative_name']);b=('@startuml\nparticipant Cedar\nparticipant Birch\nCedar -> Birch ++ : begin\nBirch -> Cedar : body\n'+note+'\nreturn after\n@enduml\n').encode();Path(n['source']).write_bytes(b);n['source_sha256']=hashlib.sha256(b).hexdigest();n['axes']=['multiline via escaped Display versus block','reverse arrow and active endpoint'];cases.append(n)
request['purpose']='Independent boundary discrimination: separate style/alignment from escaped multiline Display handling';request['cases']=cases;p=c/'supplement-request.json';p.write_text(json.dumps(request,indent=2)+'\n');p.chmod(0o444)
for n in cases:Path(n['source']).chmod(0o444)
print(len(cases),p,hashlib.sha256(p.read_bytes()).hexdigest())
