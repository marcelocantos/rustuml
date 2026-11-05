# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,json
from pathlib import Path
C=Path(__file__).parent;b=json.loads((C/'fresh-before/measurements.json').read_text());a=json.loads((C/'fresh-after/measurements.json').read_text());assert len(a)==len(b)==534
modes=['cli_comparison','product_library','assisted_library']
def category(r):
 if not r['observed_valid']:return r['reference_outcome']
 if r['cli']['exit']!=0:return 'valid_error'
 return 'match' if r['cli_comparison']['exit']==0 else 'svg_mismatch'
def stats(rows):return dict(entries=len(rows),categories=dict(collections.Counter(category(r) for r in rows)),cli_svg_panics=[r['name'] for r in rows if 'panicked at' in r['cli']['stderr']],library_panics=[r['name'] for r in rows if 'panicked at' in r['product_library']['stderr']])
res=dict(before=stats(b),after=stats(a),mechanisms={},recoveries={},losses={},new_valid_errors=[x['name'] for x,y in zip(a,b) if category(x)=='valid_error' and category(y)!='valid_error'],intended_observed_disagreements=[dict(name=x['name'],intended_valid=x['intended_valid'],observed_valid=x['observed_valid'],reference_outcome=x['reference_outcome']) for x in a if x['intended_valid']!=x['observed_valid']],entries=len(a),unique=len({x['source_sha256'] for x in a}))
for mode in modes:
 res['recoveries'][mode]=[x['name'] for x,y in zip(a,b) if x['observed_valid'] and x[mode]['exit']==0 and y[mode]['exit']!=0]
 res['losses'][mode]=[x['name'] for x,y in zip(a,b) if x['observed_valid'] and y[mode]['exit']==0 and x[mode]['exit']!=0]
for mech in sorted({x['mechanism'] for x in a}):
 before=[x for x in b if x['mechanism']==mech];after=[x for x in a if x['mechanism']==mech]
 res['mechanisms'][mech]=dict(before=stats(before),after=stats(after),whole_output_failures=[x['name'] for x in after if x['observed_valid'] and category(x)!='match'],new_valid_errors=[x['name'] for x,y in zip(after,before) if category(x)=='valid_error' and category(y)!='valid_error'],lost_matches=[x['name'] for x,y in zip(after,before) if category(y)=='match' and category(x)!='match'],unchanged_svg=[x['name'] for x,y in zip(after,before) if x['cli']['svg_sha256']==y['cli']['svg_sha256']],verdict='rejected')
# Verify model preservation independently of the SVG path.
models={label:json.loads(gzip.decompress((C/(label+'-fresh-request-cases-models.json.gz')).read_bytes())) for label in ['before','after']}
res['self_note_model_changes']=[];res['self_note_controls_changed']=[]
for x,y,m,n in zip(a,b,models['before'],models['after']):
 assert x['source']==y['source']==m['source']==n['source']
 if x['mechanism']=='sequence-teoz-self-note-component':
  if any(m['results'][k]['stdout']!=n['results'][k]['stdout'] or m['results'][k]['exit']!=n['results'][k]['exit'] for k in ['ast','yaml','ascii']):res['self_note_model_changes'].append(x['name'])
  if ('_legacy' in x['name'] or '_over_' in x['name'] or '_nonself_' in x['name']) and x['cli']['svg_sha256']!=y['cli']['svg_sha256']:res['self_note_controls_changed'].append(x['name'])
neg=json.loads((C/'fresh-rejections.json').read_text());by_source={x['source']:x for x in a};res['rejections']={}
for outcome in ['java_invalid','java_export_error']:
 rows=[r for r in neg['results'] if by_source[r['source']]['reference_outcome']==outcome];res['rejections'][outcome]=dict(entries=len(rows),before_rejections=sum(r['results']['before']['exit']!=0 for r in rows),after_rejections=sum(r['results']['after']['exit']!=0 for r in rows),lost_rejections=[r['source'] for r in rows if r['introduced_rejection_loss']],false_accepts=[r['source'] for r in rows if r['results']['after']['exit']==0])
(C/'fresh-summary.json').write_text(json.dumps(res,indent=2)+'\n');print(json.dumps({k:v for k,v in res.items() if k not in ['mechanisms','recoveries','intended_observed_disagreements','rejections']},indent=2));print('per mechanism',{k:(v['before']['categories'],v['after']['categories']) for k,v in res['mechanisms'].items()});print('rejections',{k:{kk:len(vv) if isinstance(vv,list) else vv for kk,vv in v.items()} for k,v in res['rejections'].items()})
