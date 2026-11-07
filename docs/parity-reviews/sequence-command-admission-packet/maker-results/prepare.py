import json,gzip,shutil,hashlib
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();old=R/'docs/parity-reviews/sequence-command-contract-repair-packet/maker-results';review=old.parent/'checker-2b056ca1-astra';rows=json.loads((old/'all-inputs.json').read_text());fresh=json.loads(gzip.decompress((review/'fresh-all.json.gz').read_bytes()));print(type(fresh),len(fresh));assert len(rows)==3111 and len(fresh)==422
for r in fresh:r['cohort']='checker422'
(C/'all3533.json').write_text(json.dumps(rows+fresh,indent=2)+'\n');(C/'checker422.json').write_text(json.dumps(fresh,indent=2)+'\n');(C/'before-bin').mkdir()
for p in Path('/Users/marcelo/Library/Caches/rustuml-sequence-command-9784/final-bin').iterdir():shutil.copy2(p,C/'before-bin'/p.name)
b=json.loads((old/'binaries.json').read_text())['after'];assert all(hashlib.sha256((C/'before-bin'/n).read_bytes()).hexdigest()==h for n,h in b['binaries'].items());(C/'before-binding.json').write_text(json.dumps(b,indent=2)+'\n')
