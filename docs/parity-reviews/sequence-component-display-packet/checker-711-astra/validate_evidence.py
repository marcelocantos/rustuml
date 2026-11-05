# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,pathlib,subprocess,tarfile,xml.etree.ElementTree as ET
R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-5c023380');O=pathlib.Path(__file__).parent;D=R/'docs/parity-reviews/sequence-component-display-packet/checker-711-astra'
def sha(b):return hashlib.sha256(b).hexdigest()
def read(n):return json.loads((D/n).read_text())
p=read('preservation-and-chronology.json'); protected={**p['prior_blobs'],**p['imported_blobs']};bad=[]
for path,blob in protected.items():
 data=(R/path).read_bytes(); actual=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
 if actual!=blob:bad.append(path)
assert not bad,bad
assert subprocess.check_output(['git','diff','--name-only','711dbca7'],cwd=R).decode()==''
index=read('review-index.json');assert len(index)==9
names=set()
for item in index:
 review=json.loads((R/item['review']).read_text());assert review['verdict']=='rejected'
 for key in ['reviewer','account_commit','implementation_commit','java_revision','commands','perturbations','findings']:assert review[key]
 for entry in review['perturbations']:
  src=R/entry['source'];svg=R/entry['golden'];assert src.with_suffix('.svg')==svg;assert sha(src.read_bytes())==entry['source_sha256'];assert sha(svg.read_bytes())==entry['reference_sha256'];ET.fromstring(svg.read_bytes());assert entry['result']==('pass' if entry['comparison_exit']==0 else 'fail');names.add(entry['source'])
assert len(names)==64
for version in ['before','intermediate','after']:
 binding=json.loads(gzip.decompress((D/(version+'-binding.json.gz')).read_bytes()));assert binding['status']==''
 for binary,h in binding['binaries'].items():assert sha((O/(version+'-bin')/binary).read_bytes())==h
 if version=='after':
  for path,h in binding['files'].items():assert sha((R/path).read_bytes())==h
with tarfile.open(D/'execution-captures.tar.gz') as tar:
 for folder in ['before2083','after2083','before-fresh','after-fresh','before-followup','after-followup','before-queue','intermediate-queue','after-queue']:
  rows=json.loads(tar.extractfile(folder+'/measurements.json').read())
  for row in rows:assert sha(tar.extractfile(folder+'/'+row['cli']['svg']).read())==row['cli']['svg_sha256']
summary=dict(review_records=9,unique_fresh_pairs=64,protected_paths=len(protected),protected_changes=bad,binary_source_bindings='all three exact independent builds verified; after source matches worktree',capture_integrity='Every saved CLI SVG in every replay archive matches its measurement SHA256',production_diff='none',verification='Evidence metadata/hash/schema checks only; no broad suite rerun after registration')
(D/'evidence-validation.json').write_text(json.dumps(summary,indent=2)+'\n');print(summary)
files={str(x.relative_to(D)):sha(x.read_bytes()) for x in sorted(D.rglob('*')) if x.is_file() and x.name!='file-hashes.json'}
(D/'file-hashes.json').write_text(json.dumps(files,indent=2)+'\n')
