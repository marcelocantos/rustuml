# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,re,subprocess
from pathlib import Path
C=Path(__file__).parent/'independent';R=Path.cwd();read=lambda p:json.loads(p.read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
b=read(C/'packet2371-before/measurements.json');a=read(C/'packet2371-after/measurements.json');assert len(a)==len(b)==2371
modes=['cli_comparison','product_library','assisted_library'];ranges=[('frozen',0,2281),('imported_checker',2281,2345),('fresh_relay',2345,2369),('creole_handoffs',2369,2371)]
def stats(rows):
 return dict(entries=len(rows),modes={m:dict(collections.Counter(x[m]['exit'] for x in rows)) for m in modes},cli_exits=dict(collections.Counter(x['cli']['exit'] for x in rows)),svg_panics=[x['source'] for x in rows if 'panicked at' in x['cli']['stderr']],nonpositive_cli_exits=dict(collections.Counter(x['cli']['exit'] for x in rows if x['cli_comparison']['exit']==2)))
summary=dict(entries=len(a),unique_sources=len({x['source_sha256'] for x in a}),duplicates=len(a)-len({x['source_sha256'] for x in a}),before=stats(b),after=stats(a),cohorts={n:dict(before=stats(b[i:j]),after=stats(a[i:j])) for n,i,j in ranges},losses={m:[x['source'] for x,y in zip(a,b) if y[m]['exit']==0 and x[m]['exit']!=0] for m in modes},recoveries={m:[x['source'] for x,y in zip(a,b) if y[m]['exit'] not in [0,2] and x[m]['exit']==0] for m in modes},new_valid_cli_errors=[x['source'] for x,y in zip(a,b) if x['cli_comparison']['exit']!=2 and y['cli']['exit']==0 and x['cli']['exit']!=0])
for label,rows,name in [('before',b,'before-measurements.json.gz'),('after',a,'measurements.json.gz')]:
 maker=json.loads(gzip.decompress((R/'docs/parity-reviews/sequence-filtered-display-packet/maker-results'/name).read_bytes()));assert len(maker)==len(rows)
 summary.setdefault('maker_reproduction',{})[label]=dict(source_and_reference_equal=all((x['source_sha256'],x['golden_sha256'])==(y['source_sha256'],y['golden_sha256']) for x,y in zip(rows,maker)),status_differences=[dict(index=i,source=x['source'],mode=m,ours=x[m]['exit'],maker=y[m]['exit']) for i,(x,y) in enumerate(zip(rows,maker)) for m in modes if x[m]['exit']!=y[m]['exit']],cli_svg_byte_differences=[x['source'] for x,y in zip(rows,maker) if x['cli']['svg_sha256']!=y['cli']['svg_sha256']])
mb=read(C/'packet2371-before-models.json');ma=read(C/'packet2371-after-models.json');assert len(ma)==len(mb)==2371
summary['models']=dict(yaml_by_cohort={n:dict(entries=j-i,equal=sum(x['results']['yaml']['stdout']==y['results']['yaml']['stdout'] for x,y in zip(ma[i:j],mb[i:j]))) for n,i,j in ranges},yaml_changes=[dict(index=i,source=x['source']) for i,(x,y) in enumerate(zip(ma,mb)) if x['results']['yaml']['stdout']!=y['results']['yaml']['stdout']],ascii_changes=[dict(index=i,source=x['source']) for i,(x,y) in enumerate(zip(ma,mb)) if (x['results']['ascii']['exit'],x['results']['ascii']['stdout'])!=(y['results']['ascii']['exit'],y['results']['ascii']['stdout'])],yaml_panics_before=[i for i,x in enumerate(mb) if x['results']['yaml']['exit']==101],yaml_panics_after=[i for i,x in enumerate(ma) if x['results']['yaml']['exit']==101])
legacy=[]
for i,row in enumerate(mb):
 y=row['results']['yaml']
 if y['exit']!=0:continue
 p=subprocess.run([str(C/'roundtrip')],input=y['stdout'].encode(),capture_output=True)
 legacy.append(dict(index=i,exit=p.returncode,stdout_exact=p.stdout==y['stdout'].encode(),stderr=p.stderr.decode()))
summary['legacy_roundtrip']=dict(entries=len(legacy),exact=sum(x['stdout_exact'] for x in legacy),failures=[x for x in legacy if not x['stdout_exact'] or x['exit']]);(C/'legacy-roundtrip.json').write_text(json.dumps(legacy,indent=2)+'\n')
maker_models=json.loads(gzip.decompress((R/'docs/parity-reviews/sequence-filtered-display-packet/maker-results/final-model-and-ascii.json.gz').read_bytes()))
summary['models']['maker_stdout_or_status_differences']=[dict(index=i,revision=label,mode=mode) for label,rows in [('before',mb),('after',ma)] for i,(x,y) in enumerate(zip(rows,maker_models)) for mode in ['model','ascii','yaml'] if (x['results'][mode]['exit'],x['results'][mode]['stdout'])!=(y['results'][label][mode]['exit'],y['results'][label][mode]['stdout'])]
(C/'packet-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k not in ['recoveries','models','cohorts']},indent=2));print('model changes',len(summary['models']['yaml_changes']),len(summary['models']['ascii_changes']));print('recoveries',{k:len(v) for k,v in summary['recoveries'].items()})
