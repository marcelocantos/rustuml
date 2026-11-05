# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package immutable maker captures; does not invoke any oracle or alter prior evidence."""
import gzip,hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
R=Path.cwd();S=Path('/Users/marcelo/Library/Caches/rustuml-tim7-maker');D=R/'docs/parity-reviews/tim-exact-memory-identities/maker-support';D.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,x):p.write_text(json.dumps(x,indent=2)+'\n')
# Retain the exact immutable before-membership metadata the replay consumed.
P=Path('/Users/marcelo/Library/Caches/rustuml-review-4a288dd-checker6');I=S/'replay-inputs';I.mkdir(exist_ok=True)
for suite in ['collateral','prior31','fresh29','maker15','maker17','maker18','fresh6']:
 name='fresh-results.json' if suite=='fresh6' else suite+'-results.json'
 shutil.copy2(P/name,I/name)
# Source/probe references at original ephemeral paths remain reconstructible from bytes.
inputs={}
for p in I.glob('*.json'):
 for row in json.loads(p.read_bytes()):
  for key in ['source','golden','reference_source']:
   if key not in row:continue
   source=Path(row[key]);source=source if source.is_absolute() else R/source
   if not source.exists():continue
   b=source.read_bytes();h=sha(b);inputs[str(source)]={'sha256':h,'bytes':len(b)}
   dest=S/'replay-input-bytes'/h;dest.parent.mkdir(exist_ok=True)
   if not dest.exists():dest.write_bytes(b)
dump(S/'replay-input-paths.json',inputs)
for n in ['analysis.json','negative-statuses.json','integrity2.json','gates2.json','signature-capacity-derivation.json','java-source-provenance.json','unit-expectation-corrections.json','filename-regression.json','controls2.json']:
 shutil.copy2(S/n,D/n)
for p in S.glob('final2-*-summary.json'):shutil.copy2(p,D/p.name)
for folder,destname,sourcefolder in [('relay15-final2','relay15','inputs'),('relay2-final2','relay2','supplement')]:
 dest=D/destname;dest.mkdir(exist_ok=True);out=D/sourcefolder;out.mkdir(exist_ok=True)
 for row in json.loads((S/folder/'results.json').read_bytes()):
  p=Path(row['source']);c=S/folder/p.stem;t=dest/p.stem;t.mkdir(exist_ok=True)
  shutil.copy2(p,out/p.name);shutil.copy2(c/'java.svg.stdout',out/p.with_suffix('.svg').name)
  for f in c.iterdir():
   if f.is_file():shutil.copy2(f,t/f.name)
 for f in (S/folder).iterdir():
  if f.is_file():shutil.copy2(f,dest/f.name)
for name in ['new-manifest.json','remote-request.json','supplement-manifest.json','supplement-request.json','replay.py','relay_eval.py','relay2_eval.py','analyze.py','gates2.py','integrity2.py','controls2.py','filename_check.py','package.py']:
 shutil.copy2(S/name,D/name)
# No executables or build outputs in Git. Preserve all failed and final measured bytes.
excluded={'target-after','before-rustuml','iteration1-rustuml','final-rustuml','final2-rustuml','probe'}
with (D/'measurements.tar.gz').open('wb') as raw:
 with gzip.GzipFile(fileobj=raw,mode='wb',mtime=0) as gz:
  with tarfile.open(fileobj=gz,mode='w|') as tar:
   for p in sorted(S.iterdir()):
    if p.name not in excluded:tar.add(p,arcname=p.name)
# README and this inventory are added after the narrative is written.
print(D,sha((D/'measurements.tar.gz').read_bytes()))
