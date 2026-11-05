# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent actual-CLI replay against immutable references and comparator."""
import concurrent.futures, hashlib, importlib.util, json, os, subprocess, sys
from pathlib import Path
ROOT=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-cb14b5f3');S=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3')
module=ROOT/'docs/parity-reviews/tim-typed-values-and-binding/checker3-support/check.py'
spec=importlib.util.spec_from_file_location('prior_checker',module);c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
c.S=S;c.ROOT=ROOT
c.ENV.update(CARGO_TARGET_DIR=str(S/'target'),CARGO_BUILD_JOBS='4')
# Preserve the prior per-case environment contract. All diagnostics remain supplemental.
if sys.argv[1]=='collateral':
 rows=json.loads((S/'collateral-manifest.json').read_text())
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
  results=list(pool.map(lambda row:c.measure(row,'candidate',row['cohort']!='frozen'),rows))
 (S/'collateral-results.json').write_text(json.dumps(results,indent=2)+'\n')
 old={r['source']:r for r in json.loads((S/'baseline-results.json').read_text())}
 summary=dict(before=c.summary(list(old.values())),after=c.summary(results),gains=[r['source'] for r in results if r['svg_match'] and not old[r['source']]['svg_match']],losses=[r['source'] for r in results if not r['svg_match'] and old[r['source']]['svg_match']],new_valid_errors=[r['source'] for r in results if r['cli_exit'] and not r['reference_invalid'] and not old[r['source']]['cli_exit']],new_panics=[r['source'] for r in results if r['panic'] and not old[r['source']]['panic']])
 summary['cohorts']={k:dict(before=c.summary([r for r in old.values() if r['cohort']==k]),after=c.summary([r for r in results if r['cohort']==k])) for k in ['frozen','fresh','retained','supplement']}
 for k in ['cohort955','cohort1044']:summary['cohorts'][k]=dict(before=c.summary([r for r in old.values() if r.get(k)]),after=c.summary([r for r in results if r.get(k)]))
 (S/'collateral-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
