# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import runpy,sys,json,concurrent.futures,hashlib,subprocess
from pathlib import Path
from collections import Counter,defaultdict
root=Path.cwd(); support=root/'docs/parity-reviews/class-command-grammar-closure/checker-support'
g=runpy.run_path(str(support/'scripts/checker2740_run.py'))
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
binary=Path(sys.argv[2]);g['run_one'].__globals__.update(OUT=out,BINARY=binary,COMPARATOR=Path('/tmp/rustuml-cff-check-svg'))
old={r['relative'] for r in json.loads((support/'current-results.json').read_text())['results']}
base=root/'test-diagrams/perturbations'
items=[('current955' if str(p.relative_to(base)) in old else 'retained237',p,p.relative_to(base)) for p in sorted(base.rglob('*.puml'))]
if len(sys.argv)>3:
 items=[item for item in items if sys.argv[3] == item[0] or sys.argv[3] in str(item[2])]
else: assert len(items)==1192
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool: rows=list(pool.map(g['run_one'],items))
counts=defaultdict(Counter)
for row in rows: counts[row['cohort']][row['status']]+=1
report={'tested_revision':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'cwd':str(root),'counts':dict(counts),'total':dict(Counter(r['status'] for r in rows)),'results':rows}
(out/'results.json').write_text(json.dumps(report,indent=2)+'\n');print(report['counts'],report['total'])
