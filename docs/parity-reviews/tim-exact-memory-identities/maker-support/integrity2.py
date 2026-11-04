# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,subprocess
from pathlib import Path
R=Path.cwd();S=Path('/Users/marcelo/Library/Caches/rustuml-tim7-maker');old='ba57d3c7';paths=['docs/parity-reviews','test-diagrams/perturbations']
entries=subprocess.check_output(['git','ls-tree','-rz',old,'--',*paths]).split(b'\0');fail=[];n=0
for entry in entries:
 if not entry:continue
 metadata,path=entry.split(b'\t');mode,kind,sha=metadata.split();p=R/path.decode();data=p.read_bytes();actual=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest();n+=1
 if actual!=sha.decode():fail.append(str(p))
assert not fail,fail
out=dict(unchanged_import_revision=subprocess.check_output(['git','rev-parse',old],text=True).strip(),old_evidence_files_verified=n,modified_old_files=fail,candidate=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),golden_gitlink=subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD'],text=True).strip())
out['binaries']={name:dict(path=str(S/name),sha256=hashlib.sha256((S/name).read_bytes()).hexdigest()) for name in ['before-rustuml','iteration1-rustuml','final-rustuml','final2-rustuml','probe']}
out['module_sha256']=hashlib.sha256((R/'crates/rustuml-parser/src/preprocess/mod.rs').read_bytes()).hexdigest()
out['before_module_sha256']=hashlib.sha256(subprocess.check_output(['git','show','4a288dd:crates/rustuml-parser/src/preprocess/mod.rs'])).hexdigest()
out['build_targets']={'before':'/tmp/rustuml-tim6-target (unchanged 4a288dd candidate binary; copied with recorded d12f8b9b hash)' ,'current':'/Users/marcelo/Library/Caches/rustuml-tim7-maker/target-after (fresh isolated persistent target, jobs4; no source switch)'}
(S/'integrity2.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out,indent=2))
