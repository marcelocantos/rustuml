# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import runpy,json,concurrent.futures,sys,hashlib
from collections import Counter
g=runpy.run_path('docs/parity-reviews/class-command-grammar-closure/checker-support/scripts/checker2740_run.py')
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True);binary=Path(sys.argv[2]);g['run_one'].__globals__.update(OUT=out,BINARY=binary,COMPARATOR=Path('/tmp/rustuml-cff-check-svg'))
base=Path('/tmp/rustuml-cff3/priority-fresh');items=[('fresh24',p,p.relative_to(base)) for p in sorted(base.glob('*.puml'))]
assert len(items)==4 and all(p.with_suffix('.svg').exists() for _,p,_ in items)
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool: rows=list(pool.map(g['run_one'],items))
report={'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'counts':dict(Counter(r['status'] for r in rows)),'results':rows}
(out/'results.json').write_text(json.dumps(report,indent=2)+'\n');print(report['counts'])
