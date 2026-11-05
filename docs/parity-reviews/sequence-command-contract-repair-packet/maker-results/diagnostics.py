# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,collections,hashlib
from pathlib import Path
C=Path(__file__).parent;read=lambda p:json.loads(p.read_text());write=lambda n,v:(C/n).write_text(json.dumps(v,indent=2)+'\n')
a=read(C/'before3111/measurements.json');b=read(C/'final3111/measurements.json');assert len(a)==len(b)==3111
prior={Path(r['source']).name:r for r in read(Path('docs/parity-reviews/sequence-consumer-contract-packet/maker-results/nonpositive-contract-inputs.json'))};fresh={r['source']:r for r in read(C/'checker534.json')};new={r['name']:r for r in read(C/'reference-classification.json')};neg=[]
for x,y in zip(a,b):
 assert x['source']==y['source'] and x['source_sha256']==y['source_sha256'] and x['golden_sha256']==y['golden_sha256']
 if y['cli_comparison']['exit']!=2:continue
 meta=fresh.get(y['source'],prior.get(Path(y['source']).name))
 if meta is None:meta=new[Path(y['source']).stem];kind='java_invalid'
 else:kind=meta.get('reference_outcome','historical_nonpositive')
 neg.append(dict(source=y['source'],reference_kind=kind,intended_valid=meta.get('intended_valid'),java_exit=meta.get('java_exit'),before=x['cli'],after=y['cli'],rejection_before=x['cli']['exit']!=0,rejection_after=y['cli']['exit']!=0,lost_rejection=x['cli']['exit']!=0 and y['cli']['exit']==0))
write('rejection-contract.json',dict(entries=len(neg),kinds={k:dict(entries=len(rs),before_rejected=sum(r['rejection_before'] for r in rs),after_rejected=sum(r['rejection_after'] for r in rs),false_successes=[r['source'] for r in rs if not r['rejection_after']]) for k in sorted({r['reference_kind'] for r in neg}) for rs in [[r for r in neg if r['reference_kind']==k]]},lost_rejections=[r['source'] for r in neg if r['lost_rejection']],results=neg,note='Actual CLI captures already executed in the replay; no rerender. The eight Java exporter errors are nonpositive but NOT syntax rejection obligations. Historical99 retain their previous nonpositive classification without asserting unavailable Java validity. Full SVG stdout files are in captures.tar.gz and bound by these hashes.'))
models=[]
for x,y in zip(read(C/'before-all3111-models.json'),read(C/'final-all3111-models.json')):
 assert x['source']==y['source'];models.append(dict(source=x['source'],source_sha256=x['source_sha256'],results={'before':x['results'],'after':y['results']},ascii_equal=all(x['results']['ascii'][k]==y['results']['ascii'][k] for k in ['stdout','stderr','exit'])))
write('final-model-and-ascii.json',models)
mode_stats={}
for mode in ['model','ascii','yaml']:
 mode_stats[mode]=dict(before_exits=dict(collections.Counter(x['results']['before'][mode]['exit'] for x in models)),after_exits=dict(collections.Counter(x['results']['after'][mode]['exit'] for x in models)),changed_stdout=[x['source'] for x in models if x['results']['before'][mode]['stdout']!=x['results']['after'][mode]['stdout']],new_errors=[x['source'] for x in models if x['results']['before'][mode]['exit']==0 and x['results']['after'][mode]['exit']!=0],new_panics=[x['source'] for x in models if x['results']['before'][mode]['exit']!=101 and x['results']['after'][mode]['exit']==101])
write('model-summary.json',dict(entries=len(models),frozen2549_stdout_identical=all(x['results']['before'][m]['stdout']==x['results']['after'][m]['stdout'] for x in models[:2549] for m in ['model','ascii','yaml']),modes=mode_stats,note='Every stdout/stderr/status retained. New nonzero statuses for Java-invalid accross are corrected rejections, not valid-input errors. Existing nested-enum YAML serialization panics are counted separately; no ASCII behavior-invariance or acceptance claim.'))
print('negative',len(neg),'lost rejection',sum(r['lost_rejection'] for r in neg),'models',len(models))
