# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Freeze authored review evidence; exclude executables and build targets."""
import gzip,hashlib,json,shutil,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();E=R/'docs/parity-reviews/sequence-command-admission-packet/checker-bd08723a-astra'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for directory in ['support','logs','relay-main','relay-supplement']: (E/directory).mkdir(exist_ok=True)
for p in C.glob('*.py'):shutil.copy2(p,E/'support'/p.name)
large={'fresh-before.json','fresh-after.json','fresh-main-reference-observations.json','fresh-supplement-reference-observations.json','strict-xml.json','fresh-xml.json','model-changes.json'}
skip={'reference-results.json','reference-results.json.gz','reference-manifest.json','reference-input-bundle.json.gz','reference-transport.json','parent-relay-input.json.gz','parent-relay-validation.json','parent-upload-validation.json'}
for p in C.iterdir():
    if not p.is_file() or p.name in skip:continue
    if p.suffix=='.json':
        if p.name in large:(E/(p.name+'.gz')).write_bytes(gzip.compress(p.read_bytes(),mtime=0))
        else:shutil.copy2(p,E/p.name)
    elif p.suffix=='.log':(E/'logs'/(p.name+'.gz')).write_bytes(gzip.compress(p.read_bytes(),mtime=0))
    elif p.suffix=='.diff':shutil.copy2(p,E/p.name)
for src,dest in [(C,E/'relay-main'),(C/'supplement',E/'relay-supplement')]:
    for name in skip:
        p=src/name
        if p.exists() and name!='reference-results.json':shutil.copy2(p,dest/name)
for p in (C/'gates').iterdir():
    if p.suffix=='.log':(E/'logs'/('broad-'+p.name+'.gz')).write_bytes(gzip.compress(p.read_bytes(),mtime=0))
    else:shutil.copy2(p,E/p.name)
# Full stdout+stderr+exit/argv records. Sources and Java SVGs are separately
# preserved byte-exact in standing perturbations; binaries are never archived.
dirs=['retained-before','retained-after','retained-before-full','retained-after-full','golden-before-full','golden-after-full','fresh-main-before','fresh-main-after','fresh-supplement-before','fresh-supplement-after','fresh-main-before-full','fresh-main-after-full','fresh-supplement-before-full','fresh-supplement-after-full']
archive=C/'captures.tar.gz'
with tarfile.open(archive,'w:gz',compresslevel=6) as tar:
    for name in dirs:tar.add(C/name,arcname=name)
shutil.copy2(archive,E/archive.name)
artifacts=[dict(path=str(p.relative_to(E)),bytes=p.stat().st_size,sha256=sha(p)) for p in sorted(E.rglob('*')) if p.is_file() and p.name!='artifact-integrity.json']
(E/'artifact-integrity.json').write_text(json.dumps(dict(artifacts=artifacts,excluded=['release binaries','Cargo targets','Python bytecode'],fresh_source_pairs='test-diagrams/perturbations/*/bd087*.puml and same-stem .svg'),indent=2)+'\n')
print('files',len(artifacts),'archive bytes',archive.stat().st_size,flush=True)
