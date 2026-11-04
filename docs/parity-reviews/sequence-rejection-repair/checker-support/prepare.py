# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Verify immutable evidence and prepare isolated replay inputs; no Java execution."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[4]
OUT = Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-d382aba6')
REPORT = Path(__file__).resolve().parent
MAKER = ROOT / 'docs/parity-reviews/sequence-rejection-repair/maker-results'
CANDIDATE = '0fb4e14b7227eb768d268128d42fa5f6d5a5b1af'
PARENT = 'caae198aabd7a20a989e906b42749602a1d73cc5'

def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)

def sha(b):
    return hashlib.sha256(b).hexdigest()

def save(name, value):
    (REPORT / name).write_text(json.dumps(value, indent=2)+'\n')

manifest = json.loads((MAKER/'account-normalization/manifest.json').read_text())
normalized = []
for r in manifest['records']:
    original = git('cat-file', 'blob', r['original_git_blob'])
    assert git('show', r['original_commit']+':'+r['path']) == original
    assert (ROOT/r['original_bytes_archive']).read_bytes() == original
    assert sha(original) == r['original_sha256']
    old = json.loads(original)
    expected = dict(old, schema_version=1, mechanism=Path(r['path']).parent.name)
    for src, dst in manifest['field_copy_mapping'].items():
        expected[dst] = old[src]
    current = (ROOT/r['path']).read_bytes()
    assert json.loads(current) == expected
    assert sha(current) == r['normalized_sha256']
    commits = git('log','--reverse','--format=%H','--diff-filter=A','--',r['path']).decode().splitlines()
    assert commits[0] == r['original_commit']
    subprocess.run(['git','merge-base','--is-ancestor',r['original_commit'],'82914165'],cwd=ROOT,check=True)
    normalized.append(dict(r, exact_original_verified=True, field_mapping_verified=True,
                           original_add_commit_verified=True, ancestor_of_final_implementation=True))
save('normalization-audit.json',normalized)

binaries = json.loads((MAKER/'binaries.json').read_text())
for p,h in binaries['source_files'].items():
    assert sha((ROOT/p).read_bytes())==h,p
for name,h in binaries['binaries'].items():
    p=Path('/Users/marcelo/Library/Caches/rustuml-sequence-repair/bin')/name
    assert sha(p.read_bytes())==h,name
    shutil.copy2(p,OUT/'bin'/name)
save('reused-binaries.json',dict(binaries,verification='Every recorded binary and source SHA256 checked before copy; these early replays use maker binary. Final gates use independently rebuilt candidate.'))

items=json.loads((MAKER/'all-inputs.json').read_text())
assert len(items)==1728
for r in items:
    assert (ROOT/r['source']).is_file(),r['source']
    assert (ROOT/r['golden']).is_file(),r['golden']
(OUT/'all1728.json').write_text(json.dumps(items,indent=2)+'\n')
paths=git('ls-tree','-r','--name-only',PARENT,'--','test-diagrams/perturbations').decode().splitlines()
parent=[]
overlap=[]
for p in paths:
    dest=OUT/'parent'/p
    dest.parent.mkdir(parents=True,exist_ok=True)
    data=git('show',PARENT+':'+p)
    dest.write_bytes(data)
    if p.endswith('.puml'):
        parent.append(dict(source=str(dest),golden=str(dest.with_suffix('.svg')),cohort='parent1449',original_source=p,source_sha256=sha(data)))
        if (ROOT/p).is_file():
            overlap.append(dict(path=p,source_equal=(ROOT/p).read_bytes()==data,
                                golden_equal=(ROOT/p).with_suffix('.svg').read_bytes()==git('show',PARENT+':'+str(Path(p).with_suffix('.svg')))))
assert len(parent)==1449,len(parent)
assert all(r['source_equal'] and r['golden_equal'] for r in overlap)
(OUT/'parent1449.json').write_text(json.dumps(parent,indent=2)+'\n')
save('cohort-provenance.json',dict(candidate=CANDIDATE,collateral_parent=PARENT,maker_entries=len(items),parent_entries=len(parent),parent_paths_present_in_candidate=len(overlap),overlap=overlap,parent_manifest_recipe='prepare.py extracts immutable parent blobs into private cache; no parent production imported. Replays run at candidate repository root with candidate pinned include support.'))
print('Verified 12 normalization records, 3 binary hashes, 1728 maker entries, 1449 parent entries; shared parent paths',len(overlap))
