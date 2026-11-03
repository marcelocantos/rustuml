# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import runpy,sys,json,concurrent.futures
from collections import Counter
root=Path.cwd();g=runpy.run_path(str(root/'docs/parity-reviews/class-command-grammar-closure/checker-support/scripts/checker2740_run.py'))
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
g['run_one'].__globals__.update(ROOT=root/'crates/rustuml-oracle',OUT=out,BINARY=Path(sys.argv[2]),COMPARATOR=Path('/tmp/rustuml-cff-check-svg'))
names=set(Path('/tmp/rustuml-cff3/initial-main-failures.txt').read_text().splitlines())|set(Path('/Users/marcelo/.t3/worktrees/rustuml/t3-2740aefa/test-diagrams/no_oracle_failure_names.txt').read_text().splitlines())
items=[('main_counterexamples',(root/'test-diagrams/golden'/name).with_suffix('.puml'),Path(name+'.puml')) for name in sorted(names)]
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool: rows=list(pool.map(g['run_one'],items))
report={'counts':dict(Counter(r['status'] for r in rows)),'cwd':str(root/'crates/rustuml-oracle'),'scope':'Union of prior checker 63 failures and this packet first full main run 77 failures; actual CLI replay, unchanged comparator. Not a replacement for full main tier.','results':rows}
(out/'results.json').write_text(json.dumps(report,indent=2)+'\n');print(report['counts'])
