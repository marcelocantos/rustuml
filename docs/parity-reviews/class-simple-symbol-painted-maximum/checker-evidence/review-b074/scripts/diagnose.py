# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-review-b074');E=S/'evidence';ROOT=Path.cwd();P=ROOT/'docs/parity-reviews/class-simple-symbol-painted-maximum/maker-evidence/packet5'
sha=lambda b:hashlib.sha256(b).hexdigest()
report={};sets={}
for cohort,makerfile in [('preserved1559','inventory-v1.json.gz'),('prior36','focused-v1.json.gz'),('maker20','fresh-comparison.json')]:
 a=json.loads(gzip.decompress((E/f'{cohort}-before.json.gz').read_bytes()));b=json.loads(gzip.decompress((E/f'{cohort}-after.json.gz').read_bytes()));assert len(a['results'])==len(b['results'])
 changes=[];models=[];mismatches=[]
 for x,y in zip(a['results'],b['results']):
  assert x['source']==y['source'] and x['source_sha256']==y['source_sha256'] and x['reference_sha256']==y['reference_sha256']
  if x['status']!=y['status']:changes.append(dict(source=x['source'],before=x['status'],after=y['status']))
  if x['runs'][1]['stdout_sha256']!=y['runs'][1]['stdout_sha256']:models.append(x['source'])
  if y.get('maker_status') and y['status']!=y['maker_status']:mismatches.append(x['source'])
 sets[cohort]=set(x['source_sha256'] for x in a['results'])
 report[cohort]=dict(entries=len(a['results']),distinct_source_hashes=len(sets[cohort]),before=a['counts'],after=b['counts'],changes=changes,lost_passes=[x for x in changes if x['before']=='pass'],changed_models=models,disagreement_with_maker_status=mismatches,status_lists={v:{status:[x['source'] for x in r['results'] if x['status']==status] for status in sorted({x['status'] for x in r['results']})} for v,r in [('before',a),('after',b)]})
report['overlap']={f'{a}/{b}':len(sets[a]&sets[b]) for a in sets for b in sets if a<b}
(E/'replay-deltas.json').write_text(json.dumps(report,indent=2)+'\n')
print({k:{p:v for p,v in x.items() if p in ['before','after','lost_passes','changed_models','disagreement_with_maker_status']} for k,x in report.items() if k!='overlap'})
# Direct XML observations are diagnostic only; comparator is unchanged.
tag=lambda e:e.tag.rsplit('}',1)[-1]
def parse(p):return ET.fromstring(p.read_bytes())
def primitive_rows(root):return [(tag(e),sorted(e.attrib.items()),e.text or '') for e in root.iter() if tag(e) not in ['g','a']]
def own(root):
 parents={c:p for p in root.iter() for c in p};anchors=[]
 for e in root.iter():
  if tag(e)=='a':
   parent=parents[e];anchors.append(dict(attributes=e.attrib,parent_tag=tag(parent),parent_attributes=parent.attrib,child_tags=[tag(c) for c in e]))
 return anchors
observations=[]
for src in sorted((S/'fresh').rglob('*.puml')):
 try:
  j=parse(src.with_suffix('.svg'));a=parse(src.with_suffix('.before.svg'));b=parse(src.with_suffix('.after.svg'))
 except ET.ParseError as error:
  observations.append(dict(name=src.name,xml_diagnostic_error=str(error)));continue
 ob=dict(name=src.name,java_viewBox=j.get('viewBox'),before_viewBox=a.get('viewBox'),after_viewBox=b.get('viewBox'),java_anchors=own(j),before_anchors=own(a),after_anchors=own(b),before_after_non_group_anchor_xml_identical=primitive_rows(a)==primitive_rows(b))
 if src.name.startswith(('actor_','boundary_')):
  def data(r):
   e=next((e for e in r.iter() if tag(e)=='g' and e.get('data-qualified-name')=='Beacon'),None)
   if e is None:return None
   return dict(primitives=[dict(tag=tag(n),attributes=n.attrib,text=n.text or '') for n in e.iter() if tag(n) in ['text','path','ellipse']],max_emitted_text_baseline_plus_1_5=max([float(n.get('y'))+1.5 for n in e.iter() if tag(n)=='text'] or [0]))
  ob['primitive_observations']={k:data(v) for k,v in [('java',j),('before',a),('after',b)]}
 observations.append(ob)
(E/'fresh-direct-observations.json').write_text(json.dumps(observations,indent=2)+'\n')
