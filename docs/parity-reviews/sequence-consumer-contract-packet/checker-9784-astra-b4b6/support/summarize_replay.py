# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,re
from pathlib import Path
C=Path(__file__).parent;M=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-8eddc9af/docs/parity-reviews/sequence-consumer-contract-packet/maker-results')
b=json.loads((C/'replay-before-bound/measurements.json').read_text());a=json.loads((C/'replay-after/measurements.json').read_text());assert len(a)==len(b)==2549
modes=['cli_comparison','product_library','assisted_library']
def cat(r):
 if r['cli_comparison']['exit']==2:return 'nonpositive'
 if r['cli']['exit']!=0:return 'valid_error'
 return 'match' if r['cli_comparison']['exit']==0 else 'svg_mismatch'
def stats(rows):return dict(entries=len(rows),categories=dict(collections.Counter(cat(r) for r in rows)),panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']],library_panics=[r['source'] for r in rows if 'panicked at' in r['product_library']['stderr']])
summary=dict(before=stats(b),after=stats(a),cohorts={n:dict(before=stats(b[s:e]),after=stats(a[s:e])) for n,s,e in [('frozen',0,2371),('checker148',2371,2519),('maker30',2519,2549)]},recoveries={mode:[x['source'] for x,y in zip(a,b) if x[mode]['exit']==0 and y[mode]['exit'] not in [0,2]] for mode in modes},losses={mode:[x['source'] for x,y in zip(a,b) if y[mode]['exit']==0 and x[mode]['exit']!=0] for mode in modes},new_valid_errors=[x['source'] for x,y in zip(a,b) if cat(x)=='valid_error' and cat(y)!='valid_error'])
for label,ours,file in [('before',b,'before-measurements.json.gz'),('after',a,'measurements.json.gz')]:
 maker=json.loads(gzip.decompress((M/file).read_bytes()))
 summary[label+'_maker_differences']=[dict(index=i,source=x['source']) for i,(x,y) in enumerate(zip(ours,maker)) if x['source_sha256']!=y['source_sha256'] or x['golden_sha256']!=y['golden_sha256'] or cat(x)!=cat(y) or x['cli']['svg_sha256']!=y['cli']['svg_sha256'] or any(x[k]['exit']!=y[k]['exit'] for k in modes)]
(C/'replay-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
neg=[dict(x,intended_valid=False,observed_valid=False) for x in a if cat(x)=='nonpositive'];(C/'rejection-inputs.json').write_text(json.dumps(neg,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k not in ['recoveries','cohorts']},indent=2));print('recovery counts',{k:len(v) for k,v in summary['recoveries'].items()})
