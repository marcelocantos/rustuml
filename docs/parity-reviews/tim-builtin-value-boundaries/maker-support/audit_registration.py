# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,re
from pathlib import Path
p=Path('crates/rustuml-parser/src/preprocess/mod.rs');s=p.read_text()
registered=set(re.findall(r'"([a-z_0-9]+)"',s[s.index('fn registered_builtin('):s.index('// TLineType.simpleKeyword')]))
section=s[s.index('fn eval_builtin_values('):s.index('// Arity is taken')]
implemented=set()
for m in re.finditer(r'((?:"[a-z_0-9]+"\s*(?:\|\s*)?)+)(?:if [^\n]+)?=>',section):
 implemented.update(re.findall(r'"([a-z_0-9]+)"',m[1]))
missing=sorted(registered-implemented)
Path('/tmp/rustuml-tim5/registration-audit.json').write_text(json.dumps(dict(source=str(p),source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),registered=sorted(registered),implemented_registered=sorted(registered&implemented),registered_without_dispatch=missing,scope='Static dispatch audit; missing functions remain explicit fatal paths. No additional builtin breadth implemented. Recognition is not execution parity. Existing Rust-only extensions float/is_defined retained.'),indent=2)+'\n')
print('registered',len(registered),'implemented',len(registered&implemented),'missing',len(missing),missing)
