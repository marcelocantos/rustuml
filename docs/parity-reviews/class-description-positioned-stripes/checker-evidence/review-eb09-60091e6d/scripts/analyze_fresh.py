# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,xml.etree.ElementTree as ET
from pathlib import Path
from collections import Counter
S=Path(__file__).resolve().parent
b=json.load(open(S/'fresh-before.json'))['results'];a=json.load(open(S/'fresh-after.json'))['results'];cases=[];cohorts={}
for name in ['fresh','context','classmode']:
 m=json.load(open(S/(name+'-manifest.json')));cases+=m['cases'];cohorts[name]={c['name'] for c in m['cases']}
by={c['name']:c for c in cases};assert len(by)==len(a)==len(b)==86
N=lambda p:Path(p).stem
ns='{http://www.w3.org/2000/svg}'
def dom(p):
 r=ET.parse(p).getroot()
 def node(e):return dict(tag=e.tag.split('}')[-1],attributes=e.attrib,text=e.text)
 return dict(viewBox=r.get('viewBox'),diagram_type=r.get('data-diagram-type'),texts=[node(e) for e in r.iter(ns+'text')],owners=[node(e) for e in r.iter(ns+'g') if 'data-qualified-name' in e.attrib],anchors=[node(e) for e in r.iter(ns+'a')],filters=[node(e) for e in r.iter(ns+'feOffset')],shapes=[node(e) for e in r.iter() if e.tag in [ns+'path',ns+'ellipse',ns+'rect',ns+'polygon']])
rows=[]
for x,y in zip(b,a):
 assert x['source_sha256']==y['source_sha256'];name=N(x['source']);meta=by[name];j=json.load(open(Path(y['source']).with_suffix('.java.json')))
 row=dict(name=name,axes=meta['axes'],mechanisms=meta['mechanisms'],cohort=next(k for k,v in cohorts.items() if name in v),before=x['status'],after=y['status'],before_cli=x['cli']['exit'],after_cli=y['cli']['exit'],java_exit=j['exit_code'],source_sha256=y['source_sha256'],java_sha256=y['java_sha256'],java=dom(Path(y['source']).with_suffix('.svg')))
 for label,z in [('rust_before',x),('rust_after',y)]:
  if z['cli']['exit']==0:row[label]=dom(Path(z['outputs'])/'rust.svg')
 rows.append(row)
r=dict(total_entries=86,unique_sources=len({r['source_sha256'] for r in rows}),cohorts={k:dict(entries=len(v),before=dict(Counter(r['before'] for r in rows if r['name'] in v)),after=dict(Counter(r['after'] for r in rows if r['name'] in v))) for k,v in cohorts.items()},counts_before=dict(Counter(r['before'] for r in rows)),counts_after=dict(Counter(r['after'] for r in rows)),lost_passes=[r['name'] for r in rows if r['before']=='pass' and r['after']!='pass'],new_valid_errors=[r['name'] for r in rows if r['before'] not in ['invalid_reference','process_error'] and r['after']=='process_error'],invalid_accepted_by_candidate=[r['name'] for r in rows if r['after']=='invalid_reference' and r['after_cli']==0],rows=rows)
preserved=json.load(open(S/'preserved-after.json'))['results'];r['overlap_with_preserved1675']=len({x['source_sha256'] for x in preserved}&{x['source_sha256'] for x in rows});r['combined_entry_count']=len(preserved)+len(rows);r['combined_unique_source_count']=len({x['source_sha256'] for x in preserved}|{x['source_sha256'] for x in rows})
(S/'fresh-observations.json').write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({k:v for k,v in r.items() if k!='rows'},indent=2))
