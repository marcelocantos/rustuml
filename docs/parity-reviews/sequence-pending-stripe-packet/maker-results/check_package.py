# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,subprocess,tarfile
from pathlib import Path
R=Path.cwd();D=R/'docs/parity-reviews/sequence-pending-stripe-packet/maker-results';C=Path(__file__).parent
sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda p:json.loads(p.read_text())
rows=json.loads(gzip.decompress((D/'measurements.json.gz').read_bytes()));manifest=read(D/'all-inputs.json');assert len(rows)==len(manifest)==2169
for row,item in zip(rows,manifest):
 for key in ['source','golden']:assert sha((R/item[key]).read_bytes())==row[key+'_sha256']
for p,h in read(D/'artifact-sha256.json').items():assert sha((D/p).read_bytes())==h
with tarfile.open(D/'captures.tar.gz') as t:
 for name in ['final2169/measurements.json','before2147/measurements.json','before20/measurements.json','before-handoff/measurements.json','reference-results.json','final-model-and-ascii.json','check-pending.log']:
  assert t.extractfile(name).read()==(C/name).read_bytes()
 expected={'final2169/'+row['cli']['svg']:row['cli']['svg_sha256'] for row in rows}
 for member in t.getmembers():
  if member.name in expected:assert sha(t.extractfile(member).read())==expected.pop(member.name)
 assert not expected
for item in read(D/'implementation-map.json')['accounts']:
 for impl in item['implementation_commits']:subprocess.run(['git','merge-base','--is-ancestor',item['account_commit'],impl],check=True)
assert read(D/'summary.json')['main_library_tiers']['changes']['no_oracle_failure_names.txt']['recovered']==[]
report=dict(kind='Maker packaging integrity only; not independent acceptance',inputs_verified=2169,final_svg_capture_hashes_verified=2169,preimplementation_ancestry=True,git_production_revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),compiler=subprocess.check_output(['rustc','--version'],text=True).strip(),cargo_lock_sha256=sha((R/'Cargo.lock').read_bytes()),crate_tree=subprocess.check_output(['git','rev-parse','HEAD:crates'],text=True).strip(),private_cache=str(C))
(D/'packaging-integrity.json').write_text(json.dumps(report,indent=2)+'\n');(D/'check_package.py').write_bytes(Path(__file__).read_bytes());index={str(p.relative_to(D)):sha(p.read_bytes()) for p in sorted(D.rglob('*')) if p.is_file() and p.name!='artifact-sha256.json'};(D/'artifact-sha256.json').write_text(json.dumps(index,indent=2)+'\n');print(json.dumps(report,indent=2))
