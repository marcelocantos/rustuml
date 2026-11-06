# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import importlib.util,json,sys,concurrent.futures
from pathlib import Path
S=Path(__file__).parent;R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45')
spec=importlib.util.spec_from_file_location('replay',S/'replay.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);m.S=S;m.R=R
phase=sys.argv[1]
for cohort in sys.argv[2:] or ['fresh26','checker38','fresh15','maker18','sprites50','retained']:
 rows=json.loads((S/(cohort+'-membership.json')).read_bytes())
 def run(row):
  v=m.measure(row,phase);valid=row['membership']['reference_valid'];v['helper_reference_valid']=v['reference_valid'];v['reference_valid']=valid;v['svg_match']=valid and v['svg']['exit']==0 and v['compare']['exit']==0;return v
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(run,rows))
 (S/(cohort+'-'+phase+'.json')).write_text(json.dumps(results,indent=2)+'\n')
 print(cohort,phase,'entries',len(rows),'valid',sum(x['reference_valid'] for x in results),'pass',sum(x['svg_match'] for x in results),'panics',sum(x['panic'] for x in results),flush=True)
