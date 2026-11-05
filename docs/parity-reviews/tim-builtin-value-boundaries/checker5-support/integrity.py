# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,subprocess
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();base='47a723b6fd424a2584dc3d89ac0dc0328eeb0ae7';sha=lambda b:hashlib.sha256(b).hexdigest()
files=[]
for line in subprocess.check_output(['git','ls-tree','-r',base,'docs/parity-reviews','test-diagrams'],text=True).splitlines():
 meta,p=line.split('\t');mode,kind,oid=meta.split()
 if kind=='blob':files.append((p,oid))
q=subprocess.run(['git','hash-object','--stdin-paths'],input=''.join(p+'\n' for p,_ in files),text=True,capture_output=True,check=True);hashes=q.stdout.splitlines();assert len(hashes)==len(files);changed=[p for (p,oid),current in zip(files,hashes) if oid!=current];assert not changed
membership=json.loads(gzip.decompress((R/'docs/parity-reviews/tim-builtin-value-boundaries/maker-support/frozen-membership.json.gz').read_bytes()))
for r in membership:
 b=(R/r['path']).read_bytes();assert len(b)==r['bytes'] and sha(b)==r['sha256'],r['path']
accounts='6d46a483aad389d4535e1ba24822fdd116baf517';prod='92647b126f6dc1c83ebdc6213d0c2c0a5b3211b6';candidate='1703a2727fa364ee8004ba59a22b4c6934ee5d3d'
for a,b in [(accounts,prod),(prod,candidate)]:subprocess.run(['git','merge-base','--is-ancestor',a,b],check=True)
assert subprocess.check_output(['git','diff','--name-only',accounts,prod],text=True).splitlines()==['crates/rustuml-parser/src/preprocess/mod.rs']
production_changed=subprocess.check_output(['git','diff','--name-only',prod,candidate,'crates'],text=True);assert not production_changed
record=dict(historical_base=base,historical_files_verified=len(files),historical_changed=changed,pair_files_verified=len(membership),pairs=len(membership)//2,production_diff_paths=['crates/rustuml-parser/src/preprocess/mod.rs'],account_precedes_production=True,candidate_shipping_tree_equals_production=True,binaries={phase:sha((S/('target-'+phase)/'release/rustuml').read_bytes()) for phase in ['before','after']},production_module_sha256=sha((R/'crates/rustuml-parser/src/preprocess/mod.rs').read_bytes()),probe_sha256=sha((S/'probe').read_bytes()),java_revision=subprocess.check_output(['git','-C','/Users/marcelo/work/github.com/plantuml/plantuml','rev-parse','HEAD'],text=True).strip(),golden_gitlink=subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD'],text=True).strip(),archive_hashes={n:sha((R/'docs/parity-reviews'/p).read_bytes()) for n,p in [('maker','tim-builtin-value-boundaries/maker-support/measurement-artifacts.tar.gz'),('checker4','tim-expression-value-snapshot/checker4-support/measurement-artifacts.tar.gz')]})
(S/'integrity.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record,indent=2))
