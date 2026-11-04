# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,gzip
from pathlib import Path
from collections import Counter
S=Path(__file__).resolve().parent;R=Path.cwd();P=R/'docs/parity-reviews/class-description-positioned-stripes/maker-evidence/packet6'
b=json.loads((S/'preserved-before.json').read_text())['results'];a=json.loads((S/'preserved-after.json').read_text())['results'];assert len(a)==len(b)==1675
for x,y in zip(b,a):assert (x['source'],x['source_sha256'],x['java_sha256'])==(y['source'],y['source_sha256'],y['java_sha256'])
ranges={'preserved1559':(0,1559),'prior36':(1559,1595),'prior20':(1595,1615),'prior34':(1615,1649),'maker26':(1649,1675),'all1649':(0,1649),'all1675':(0,1675)}
summary={n:dict(entries=j-i,unique_source_hashes=len({r['source_sha256'] for r in a[i:j]}),before=dict(Counter(r['status'] for r in b[i:j])),after=dict(Counter(r['status'] for r in a[i:j]))) for n,(i,j) in ranges.items()}
sets={n:{r['source_sha256'] for r in a[i:j]} for n,(i,j) in list(ranges.items())[:5]}
summary['overlaps']={n+'/'+m:len(sets[n]&sets[m]) for i,n in enumerate(sets) for m in list(sets)[i+1:]}
summary['changes']={kind:[dict(source=x['source'],before=x['status'],after=y['status'],before_exit=x['cli']['exit'],after_exit=y['cli']['exit']) for x,y in zip(b,a) if pred(x,y)] for kind,pred in [('lost_passes',lambda x,y:x['status']=='pass' and y['status']!='pass'),('new_valid_errors',lambda x,y:x['status'] not in ['process_error','invalid_reference'] and y['status']=='process_error'),('gains',lambda x,y:x['status']!='pass' and y['status']=='pass')]}
summary['all_nonpasses']=[dict(source=x['source'],source_sha256=x['source_sha256'],before=x['status'],after=y['status'],before_cli=x['cli'],after_cli=y['cli']) for x,y in zip(b,a) if y['status']!='pass']
summary['models']={mode:dict(unchanged=sum(x[mode]['stdout_sha256']==y[mode]['stdout_sha256'] for x,y in zip(b,a)),unchanged_successful=sum(x[mode]['exit']==y[mode]['exit']==0 and x[mode]['stdout_sha256']==y[mode]['stdout_sha256'] for x,y in zip(b,a)),changed=[dict(source=x['source'],before_exit=x[mode]['exit'],after_exit=y[mode]['exit']) for x,y in zip(b,a) if x[mode]['stdout_sha256']!=y[mode]['stdout_sha256']]) for mode in ['ast','yaml']}
old=json.loads(gzip.decompress((P/'final1649.json.gz').read_bytes()))['results'];summary['maker_final1649_reproduction']=[x['source'] for x,y in zip(a,old) if x['status']!=y['status'] or x['source_sha256']!=y['source_sha256'] or x['java_sha256']!=y['java_sha256']]
summary['prior1595_models_unchanged']=sum(x['ast']['stdout_sha256']==y['ast']['stdout_sha256'] for x,y in zip(b[:1595],a[:1595]));summary['prior20_yaml_unchanged']=sum(x['yaml']['stdout_sha256']==y['yaml']['stdout_sha256'] for x,y in zip(b[1595:1615],a[1595:1615]))
nb=json.loads((S/'nonclass-before.json').read_text())['results'];na=json.loads((S/'nonclass-after.json').read_text())['results'];summary['bounded_nonclass']=dict(entries=len(nb),counts_before=dict(Counter(x['status'] for x in nb)),counts_after=dict(Counter(x['status'] for x in na)),changed_svg=[x['source'] for x,y in zip(nb,na) if x['cli']['stdout_sha256']!=y['cli']['stdout_sha256']],changed_models=[x['source'] for x,y in zip(nb,na) if x['yaml']['stdout_sha256']!=y['yaml']['stdout_sha256']])
(S/'replay-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:summary[k] for k in [*ranges,'overlaps','prior1595_models_unchanged','prior20_yaml_unchanged','bounded_nonclass']},indent=2));print('losses',len(summary['changes']['lost_passes']),'new errors',len(summary['changes']['new_valid_errors']),'gains',len(summary['changes']['gains']),'maker difference',len(summary['maker_final1649_reproduction']))
