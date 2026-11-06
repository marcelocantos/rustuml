# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import tarfile,io,json,hashlib,subprocess,shutil
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();M=R/'docs/parity-reviews/creole-parameter-candidate-grammar/maker-support'
sha=lambda b:hashlib.sha256(b).hexdigest()
a=M/'measurements.tar.gz';assert sha(a.read_bytes())=='5967d8e45b3075744356d8ac7b92a449525d67d83cf5b7608a13b32883112751'
with tarfile.open(a) as t:
 members=t.getmembers();t.extractall(S/'maker',filter='data')
(S/'archive-check.json').write_text(json.dumps({'sha256':sha(a.read_bytes()),'members':len(members)},indent=2))
for phase,rev in [('before','2608469dace1f680a7e90c0895e44e657f8aaf7f'),('after','8ca73318f584dc29d7d8ab1c91f25a21718ca713'),('initial','3d92268c4aae5f8bfd3a0e271758b6a8e5442e47')]:
 dest=S/('source-'+phase);dest.mkdir()
 data=subprocess.check_output(['git','archive',rev,'Cargo.toml','Cargo.lock','crates','vendor','test-fixtures','test-diagrams','scripts','AGENTS.md','CLAUDE.md'])
 with tarfile.open(fileobj=io.BytesIO(data)) as t:t.extractall(dest,filter='data')
 # The submodule is intentionally hydrated separately at its exact pin.
 shutil.copytree(R/'test-diagrams/golden',dest/'test-diagrams/golden',dirs_exist_ok=True,ignore=shutil.ignore_patterns('.git'))
 (dest/'REVISION').write_text(rev+'\n')
print(S)
