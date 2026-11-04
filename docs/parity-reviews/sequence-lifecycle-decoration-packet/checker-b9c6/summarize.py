# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,json
from pathlib import Path
c=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-b9c6cf04');p=Path('docs/parity-reviews/sequence-lifecycle-decoration-packet/maker-results')
def load(path):return json.loads(path.read_text())
a=load(c/'replay-after/measurements.json');b=load(c/'replay-before/measurements.json');saved=json.loads(gzip.decompress((p/'measurements.json.gz').read_bytes()));delta=json.loads(gzip.decompress((p/'before-after.json.gz').read_bytes()));assert len(a)==len(b)==len(saved)==len(delta)==1828
hash_errors=[];status_errors=[]
for old,new,archive,d in zip(b,a,saved,delta):
 assert old['source']==new['source']==archive['source']==d['source']
 for key in ('source_sha256','golden_sha256'):
  if old[key]!=new[key] or new[key]!=archive[key]:hash_errors.append([new['source'],key])
 if archive['cli']['svg_sha256']!=new['cli']['svg_sha256']:hash_errors.append([new['source'],'after CLI output'])
 if (old['cli_comparison']['exit'],new['cli_comparison']['exit'],old['cli']['exit'],new['cli']['exit'])!=(d['before_comparator'],d['after_comparator'],d['before_cli'],d['after_cli']):status_errors.append(new['source'])
s=dict(entries=len(a),before=dict(collections.Counter(r['cli_comparison']['exit'] for r in b)),after=dict(collections.Counter(r['cli_comparison']['exit'] for r in a)),gains=[r['source'] for o,r in zip(b,a) if o['cli_comparison']['exit']!=0 and r['cli_comparison']['exit']==0],lost=[r['source'] for o,r in zip(b,a) if o['cli_comparison']['exit']==0 and r['cli_comparison']['exit']!=0],corrected_negatives=[r['source'] for o,r in zip(b,a) if o['cli_comparison']['exit']==2 and o['cli']['exit']==0 and r['cli']['exit']!=0],valid_cli_errors=[r['source'] for r in a if r['cli_comparison']['exit']!=2 and r['cli']['exit']!=0],panics=[r['source'] for r in a if 'panicked' in r['cli']['stderr']],timeouts=[r['source'] for r in a if r['cli']['exit']==124],archive_status_errors=status_errors,hash_errors=hash_errors,product_library=dict(collections.Counter(r['product_library']['exit'] for r in a)),assisted_library=dict(collections.Counter(r['assisted_library']['exit'] for r in a)),valid_cli_equals_product_library=sum(r['cli_equals_library'] for r in a if r['cli_comparison']['exit']!=2),frozen955=dict(collections.Counter(r['cli_comparison']['exit'] for r in a if r.get('frozen_inventory')=='parent955.json')))
(c/'replay-summary.json').write_text(json.dumps(s,indent=2)+'\n');print({k:len(v) if isinstance(v,list) else v for k,v in s.items()})
f=load(c/'fresh-after/measurements.json');old=load(c/'fresh-before/measurements.json')
for o,r in zip(old,f):print(r['source'].split('/')[-1],r['cohort'],'before',o['cli_comparison']['exit'],'after',r['cli_comparison']['exit'],'Java',r['java_exit'],'Rust',r['cli']['exit'])
