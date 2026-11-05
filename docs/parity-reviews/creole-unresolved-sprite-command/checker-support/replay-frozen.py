# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Replay a frozen cohort without rebuilding or changing its membership.

Restore measurements.tar.gz at its recorded private root and restore any
external capture paths from capture-paths.json first. Rebuild the exact CLI
and comparator per builds.json and diagnostic-builds.json. Writes into a new
private rerun directory; never overwrites the archived measured outputs.
"""
import concurrent.futures
import importlib.util
import json
import shutil
import sys
from pathlib import Path

scratch = Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker')
repo = Path('/Users/marcelo/.t3/worktrees/rustuml/t3-9a8dea60')
phase, cohort, destination = sys.argv[1:]
assert phase in ['before', 'after']
assert cohort in ['retained', 'maker18', 'sprites50']
output = Path(destination).resolve()
assert not output.exists(), 'Use a new private output directory.'
assert scratch in output.parents, 'Keep replay output in this private cache.'
rows = json.loads((scratch / (cohort + '-membership.json')).read_bytes())
spec = importlib.util.spec_from_file_location('replay', scratch / 'replay.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
output.mkdir(parents=True)
for name in [phase + '-rustuml', 'probe']:
    shutil.copy2(scratch / name, output / name)
module.R, module.S = repo, output
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    results = list(pool.map(lambda row: module.measure(row, phase), rows))
(output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
print(len(results), sum(x['reference_valid'] for x in results), sum(x['svg_match'] for x in results))
