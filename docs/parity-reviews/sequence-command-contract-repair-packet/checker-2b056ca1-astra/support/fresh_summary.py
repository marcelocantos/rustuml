# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,hashlib,json,subprocess
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();read=lambda p:json.loads(p.read_text());rows=read(C/'fresh-pairs.json')+read(C/'supplement/fresh-pairs.json');before=read(C/'fresh358-before/measurements.json')+read(C/'fresh64-before/measurements.json');after=read(C/'fresh358-after/measurements.json')+read(C/'fresh64-after/measurements.json')
# Refine the initial status-based import taxonomy using raw stderr and pinned
# ColorParser data flow. Raw imports/captures stay unchanged and archived.
refinements=[]
for r in rows:
 if r['name'].startswith('c2b0_rich_color_'):
  refinements.append(dict(name=r['name'],initial=r['reference_outcome'],final='java_color_error',evidence='stderr: No such color; SingleLineCommand2#execute catches NoSuchColorException; source-extracted regex capture #back retained in color-capture-derivation.json'))
  r['reference_outcome']='java_color_error';r['syntax_rejection_obligation']=False
 r['command_rejection_obligation']=r['java_exit']==200
for xs in [before,after]:
 for r,meta in zip(xs,rows):
  assert r['source_sha256']==meta['source_sha256'] and r['golden_sha256']==meta['golden_sha256']
  for k in ['reference_outcome','syntax_rejection_obligation','command_rejection_obligation']:r[k]=meta[k]
def stats(xs):return dict(entries=len(xs),match=sum(x['cli_comparison']['exit']==0 for x in xs),mismatch=sum(x['cli_comparison']['exit']==1 for x in xs),nonpositive=sum(x['cli_comparison']['exit']==2 for x in xs),valid_cli_errors=sum(x['observed_valid'] and x['cli']['exit']!=0 for x in xs),cli_panics=[x['source'] for x in xs if 'panicked at' in x['cli']['stderr']])
assert len(rows)==422 and len({x['source_sha256'] for x in rows})==422
old={x['source_sha256'] for x in read(C/'after3111/measurements.json')};assert not old & {x['source_sha256'] for x in rows}
neg=[r for r in rows if r['command_rejection_obligation']];assert len(neg)==54
summary=dict(before=stats(before),after=stats(after),valid_references=sum(x['observed_valid'] for x in rows),reference_outcomes=dict(collections.Counter(x['reference_outcome'] for x in rows)),intent_disagreements=[dict(name=x['name'],intended_valid=x['intended_valid'],observed_valid=x['observed_valid'],outcome=x['reference_outcome']) for x in rows if x['intended_valid']!=x['observed_valid']],gains=[y['source'] for x,y in zip(before,after) if x['cli_comparison']['exit'] not in [0,2] and y['cli_comparison']['exit']==0],losses=[y['source'] for x,y in zip(before,after) if x['cli_comparison']['exit']==0 and y['cli_comparison']['exit']!=0],new_valid_errors=[y['source'] for x,y in zip(before,after) if y['observed_valid'] and x['cli']['exit']==0 and y['cli']['exit']!=0],lost_rejections=[y['source'] for x,y in zip(before,after) if y['command_rejection_obligation'] and x['cli']['exit']!=0 and y['cli']['exit']==0],corrected_valid_errors=[y['source'] for x,y in zip(before,after) if y['observed_valid'] and x['cli']['exit']!=0 and y['cli']['exit']==0],by_mechanism={m:dict(before=stats([x for x in before if x['mechanism']==m]),after=stats([x for x in after if x['mechanism']==m])) for m in sorted({r['mechanism'] for r in rows})},novelty=dict(entries=422,distinct_sources=422,overlap_with_3111=0,overlap_with_all_prior_and_maker_sources=0),classification_refinements=refinements)
for name,val in [('fresh-all.json',rows),('fresh-before-all.json',before),('fresh-after-all.json',after),('fresh-summary.json',summary),('negative-independent.json',neg)]: (C/name).write_text(json.dumps(val,indent=2)+'\n')
argv=['uv','run','--no-project','python','docs/parity-reviews/sequence-consumer-contract-packet/checker-9784-astra-b4b6/support/rejection_contract.py',str(R),str(C/'negative-independent.json'),str(C/'before-bin/rustuml'),str(C/'after-bin/rustuml'),str(C/'rejection-independent.json')]
with (C/'rejection-independent.log').open('wb') as f:p=subprocess.run(argv,stdout=f,stderr=subprocess.STDOUT)
(C/'rejection-independent-command.json').write_text(json.dumps(dict(argv=argv,exit=p.returncode),indent=2)+'\n')
print(json.dumps({k:summary[k] for k in ['before','after','reference_outcomes','by_mechanism','new_valid_errors','lost_rejections','corrected_valid_errors']},indent=2))
