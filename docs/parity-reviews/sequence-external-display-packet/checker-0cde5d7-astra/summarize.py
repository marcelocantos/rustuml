# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,re,subprocess,sys
from pathlib import Path
C=Path(__file__).resolve().parent;R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-5fc3056e')
def sha(b):return hashlib.sha256(b).hexdigest()
def classify(r,rev):
 v=r['results'][rev]
 if v['cli_comparison']['exit']==2:return 'invalid'
 if v['cli']['exit']!=0:return 'runtime_failure'
 return 'match' if v['cli_comparison']['exit']==0 else 'svg_mismatch'
def stats(rows):
 d=dict(entries=len(rows),distinct=len({r['source_sha256'] for r in rows}),before=dict(collections.Counter(classify(r,'before') for r in rows)),after=dict(collections.Counter(classify(r,'after') for r in rows)))
 for name,pred in [('lost_passes',lambda r:classify(r,'before')=='match' and classify(r,'after')!='match'),('recoveries',lambda r:classify(r,'before') not in ['match','invalid'] and classify(r,'after')=='match'),('new_valid_errors',lambda r:classify(r,'after')=='runtime_failure' and r['results']['before']['cli']['exit']==0),('final_panics',lambda r:r['results']['after']['cli']['exit']==101),('ast_changed',lambda r:r['results']['before']['ast']['sha256']!=r['results']['after']['ast']['sha256']),('ascii_changed',lambda r:r['results']['before']['ascii']['sha256']!=r['results']['after']['ascii']['sha256'])]:
  d[name]=[r['source'] for r in rows if pred(r)]
 d['losses_by_mode']={mode:[r['source'] for r in rows if r['results']['before'][mode]['exit']==0 and r['results']['after'][mode]['exit']!=0] for mode in ['cli_comparison','product_library','assisted_library']}
 d['invalid_rust_statuses']={rev:dict(collections.Counter(r['results'][rev]['cli']['exit'] for r in rows if classify(r,rev)=='invalid')) for rev in ['before','after']}
 return d
prefix=sys.argv[1];rows=json.loads(gzip.decompress((C/prefix/'measurements.json.gz').read_bytes()));summary=stats(rows)
if prefix=='replay2281':
 summary['cohorts']={n:stats(rows[a:b]) for n,a,b in [('frozen',0,2169),('checker96',2169,2265),('maker16',2265,2281)]}
 summary['frozen_cli_changes']=[r['source'] for r in rows[:2169] if any(r['results']['before']['cli'][k]!=r['results']['after']['cli'][k] for k in ['exit','sha256'])]
 summary['recoveries_by_previous']=dict(collections.Counter(classify(r,'before') for r in rows if classify(r,'before') not in ['match','invalid'] and classify(r,'after')=='match'))
 summary['original_accepted_scope_replays']={m:stats([r for r in rows[2169:2265] if '/'+m+'/' in r['source']]) for m in ['sequence-self-half-polygon-order','sequence-collections-source-clarification','sequence-single-pending-create']}
 summary['final_helper_panics']=[dict(source=r['source'],mode=mode,stderr=r['results']['after'][mode]['stderr']) for r in rows for mode in ['product_library','assisted_library','library_render'] if r['results']['after'][mode]['exit']==101]
else:summary['mechanisms']={m:stats([r for r in rows if r['mechanism']==m]) for m in sorted({r['mechanism'] for r in rows})}
(C/(prefix+'-summary.json')).write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k not in ['cohorts','mechanisms','final_helper_panics','original_accepted_scope_replays']},indent=2))
