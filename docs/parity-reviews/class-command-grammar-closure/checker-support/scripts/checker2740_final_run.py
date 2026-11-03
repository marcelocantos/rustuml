# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Replay every executable held-out after evidence retention, with isolated outcomes."""
from pathlib import Path
from collections import Counter, defaultdict
import concurrent.futures
import hashlib
import json
import runpy
import subprocess
import sys

root = Path.cwd()
g = runpy.run_path(str(Path(__file__).with_name('checker2740_run.py')))
output = Path('/tmp/rustuml-checker2740-final')
g['run_one'].__globals__['OUT'] = output
base = root/'test-diagrams/perturbations'
old = {r['relative'] for r in json.loads(Path('/tmp/rustuml-checker2740/current-results.json').read_text())['results']}
items = [('current955' if str(p.relative_to(base)) in old else 'retained237', p, p.relative_to(base)) for p in sorted(base.rglob('*.puml'))]
assert len(old)==955 and len(items)==1192,(len(old),len(items))
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
    rows=list(pool.map(g['run_one'],items))
counts=defaultdict(Counter)
for r in rows:
    counts[r['cohort']][r['status']]+=1
report={'tested_revision':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'production_tree':'fefca37686ff0c93b5b306831e4554a2d8f9efa6','cwd':str(root),'binary_sha256':hashlib.sha256(g['BINARY'].read_bytes()).hexdigest(),'counts':dict(counts),'total':dict(Counter(r['status'] for r in rows)),'results':rows}
(output/'results.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'counts':report['counts'],'total':report['total']},indent=2))
sys.exit(1 if any(r['status']!='pass' for r in rows) else 0)
