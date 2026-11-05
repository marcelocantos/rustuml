# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,shutil,subprocess
from pathlib import Path
C=Path(__file__).parent;T=Path('/Users/marcelo/Library/Caches/rustuml-sequence-repair/target/release');D=C/'final-bin';D.mkdir();shutil.copy2(T/'rustuml',D/'rustuml');shutil.copy2(T/'examples/diff_one',D/'diff_one');shutil.copy2(C/'before-bin/compare_cli',D/'compare_cli');sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();v=dict(revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),binaries={p.name:sha(p) for p in D.iterdir()},source_files={str(p):sha(p) for p in map(Path,['crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs'])});(C/'final-binding.json').write_text(json.dumps(v,indent=2)+'\n')
