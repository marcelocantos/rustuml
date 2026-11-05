# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,collections,gzip,hashlib,json,re
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();M=R/'docs/parity-reviews/sequence-command-contract-repair-packet/maker-results'
read=lambda p:json.loads(p.read_text())
def stats(rows):return dict(entries=len(rows),match=sum(r['cli_comparison']['exit']==0 for r in rows),mismatch=sum(r['cli_comparison']['exit']==1 for r in rows),nonpositive=sum(r['cli_comparison']['exit']==2 for r in rows),valid_errors=sum(r['cli_comparison']['exit'] not in [0,1,2] for r in rows),cli_panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']])
a=read(C/'before3111/measurements.json');b=read(C/'after3111/measurements.json');assert len(a)==len(b)==3111
for x,y in zip(a,b):assert all(x[k]==y[k] for k in ['source','source_sha256','golden_sha256'])
expected=[json.loads(gzip.decompress((M/p).read_bytes())) for p in ['before-measurements.json.gz','measurements.json.gz']]
comparisons=[]
for label,rows,ex in zip(['before','after'],[a,b],expected):
 disagreements=[]
 for i,(x,y) in enumerate(zip(rows,ex)):
  assert x['source_sha256']==y['source_sha256'] and x['golden_sha256']==y['golden_sha256']
  if any(x[m]['exit']!=y[m]['exit'] for m in ['cli','cli_comparison','product_library','assisted_library']):disagreements.append(i)
 comparisons.append(dict(label=label,disagreements=disagreements))
losses={m:[y['source'] for x,y in zip(a,b) if x[m]['exit']==0 and y[m]['exit']!=0] for m in ['cli_comparison','product_library','assisted_library']}
newerrors=[y['source'] for x,y in zip(a,b) if y['cli_comparison']['exit']!=2 and not x['cli']['exit'] and y['cli']['exit']]
oldmodels=read(C/'before-all-inputs-models.json');newmodels=read(C/'after-all-inputs-models.json');modelstats={}
for mode in ['ast','ascii','yaml']:
 xs=[x['results'][mode] for x in oldmodels];ys=[x['results'][mode] for x in newmodels]
 modelstats[mode]=dict(before_exits=dict(collections.Counter(x['exit'] for x in xs)),after_exits=dict(collections.Counter(y['exit'] for y in ys)),frozen_stdout_equal=all(x['stdout_base64']==y['stdout_base64'] for x,y in zip(xs[:2549],ys[:2549])),all_changed_stdout=sum(x['stdout_base64']!=y['stdout_base64'] for x,y in zip(xs,ys)),new_panics=[oldmodels[i]['source'] for i,(x,y) in enumerate(zip(xs,ys)) if x['exit']!=101 and y['exit']==101])
alias=[r for r in b if re.search(r'b4b6_domain_of_(spaced|tight|coloured|multi_coloured|multi|rnote|hnote|pair)_',r['source'])];assert len(alias)==16
summary=dict(before=stats(a),after=stats(b),unique_sources=len({r['source_sha256'] for r in b}),duplicate_entries=len(b)-len({r['source_sha256'] for r in b}),cohorts={name:dict(before=stats(a[lo:hi]),after=stats(b[lo:hi])) for name,lo,hi in [('frozen2549',0,2549),('checker534',2549,3083),('maker24',3083,3107),('signature4',3107,3111)]},gains=[y['source'] for x,y in zip(a,b) if x['cli_comparison']['exit'] not in [0,2] and y['cli_comparison']['exit']==0],losses=losses,new_valid_cli_errors=newerrors,maker_comparison=comparisons,model_modes=modelstats,alias16=stats(alias))
(C/'replay-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({k:v for k,v in summary.items() if k not in ['cohorts','gains']},indent=2))
