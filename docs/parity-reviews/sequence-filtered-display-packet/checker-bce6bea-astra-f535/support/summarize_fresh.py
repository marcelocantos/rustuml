# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,re,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent/'independent';R=Path.cwd();read=lambda p:json.loads(p.read_text());sha=lambda b:hashlib.sha256(b).hexdigest();datasets=['fresh130','unicode6','boundary12'];rows=[];geometry=[];ownership=[];display=[]
def elements(path):
 try:return list(ET.parse(path).getroot().iter())
 except ET.ParseError:return []
def note_shapes(path):
 result=[]
 for e in elements(path):
  if e.get('fill')!='#FEFFDD':continue
  tag=e.tag.split('}')[-1]
  if tag=='rect':x=float(e.get('x','0'));y=float(e.get('y','0'));w=float(e.get('width'));h=float(e.get('height'));bounds=[x,y,x+w,y+h]
  elif tag in ['polygon','path']:
   value=e.get('points') if tag=='polygon' else e.get('d');nums=[float(s) for s in re.findall(r'-?\d+(?:\.\d+)?',value)];xs=nums[::2];ys=nums[1::2];bounds=[min(xs),min(ys),max(xs),max(ys)]
  else:continue
  result.append(dict(element=tag,attributes=e.attrib,bounds=bounds,width=bounds[2]-bounds[0],height=bounds[3]-bounds[1]))
 return result
def notes(ast):
 result=[]
 for block in re.findall(r'\bNote \{(.*?)\n\s*\}',ast,re.S):
  participants=re.search(r'participants: \[(.*?)\n\s*\],',block,re.S);text=re.search(r'text: ("(?:\\.|[^"\\])*"),',block)
  result.append(dict(participants=re.findall(r'"([^"\n]*)"',participants[1]) if participants else [],text=json.loads(text[1]) if text else None,on_message='on_message: true' in block))
 return result
for dataset in datasets:
 bs=read(C/(dataset+'-before/measurements.json'));as_=read(C/(dataset+'-after/measurements.json'));mb=read(C/(dataset+'-before-models.json'));ma=read(C/(dataset+'-after-models.json'))
 for i,(b,a,bm,am) in enumerate(zip(bs,as_,mb,ma)):
  row=dict(name=a['name'],mechanism=a['cohort'],dataset=dataset,index=i,intended_valid=a['intended_valid'],observed_valid=a['observed_valid'],axes=a['axes'],source=a['source'],golden=a['golden'],source_sha256=a['source_sha256'],golden_sha256=a['golden_sha256'],before={m:b[m]['exit'] for m in ['cli_comparison','product_library','assisted_library']},after={m:a[m]['exit'] for m in ['cli_comparison','product_library','assisted_library']},before_cli=b['cli']['exit'],after_cli=a['cli']['exit'],before_svg_panic='panicked at' in b['cli']['stderr'],after_svg_panic='panicked at' in a['cli']['stderr'],before_yaml_exit=bm['results']['yaml']['exit'],after_yaml_exit=am['results']['yaml']['exit'])
  rows.append(row)
  paths=dict(java=Path(a['golden']),before=C/(dataset+'-before')/b['cli']['svg'],after=C/(dataset+'-after')/a['cli']['svg'])
  if a['cohort']=='sequence-note-aggregate-paint-height':
   v=dict(name=a['name'],shapes={k:note_shapes(p) for k,p in paths.items()},whole_before=b['cli_comparison']['exit'],whole_after=a['cli_comparison']['exit'])
   v['max_paint_height']={k:max((s['height'] for s in shapes),default=None) for k,shapes in v['shapes'].items()};v['height_equal_after']=abs(v['max_paint_height']['java']-v['max_paint_height']['after'])<0.000001;geometry.append(v)
  if a['cohort']=='sequence-external-note-owner':
   n_before=notes(bm['results']['model']['stdout']);n_after=notes(am['results']['model']['stdout']);text={k:[''.join(e.itertext()) for e in elements(p) if e.tag.endswith('}text')] for k,p in paths.items()};labels=[re.sub(r'<[^>]+>','',s) for n in n_after for s in n['text'].splitlines()];ownership.append(dict(name=a['name'],observed_valid=a['observed_valid'],notes_before=n_before,notes_after=n_after,event_counts=dict(before=len(n_before),after=len(n_after)),all_text=text,note_label_counts={label:{k:ts.count(label) for k,ts in text.items()} for label in labels},whole_after=a['cli_comparison']['exit']))
  if a['cohort']=='sequence-filtered-ordered-participant-display':
   display.append(dict(name=a['name'],whole_before=b['cli_comparison']['exit'],whole_after=a['cli_comparison']['exit'],text={k:[dict(text=''.join(e.itertext()),x=e.get('x'),y=e.get('y')) for e in elements(p) if e.tag.endswith('}text')] for k,p in paths.items()}))
def stats(rs):
 return dict(entries=len(rs),positive_references=sum(r['observed_valid'] for r in rs),nonpositive_references=sum(not r['observed_valid'] for r in rs),before=dict(collections.Counter(r['before']['cli_comparison'] for r in rs)),after=dict(collections.Counter(r['after']['cli_comparison'] for r in rs)),positive_match_losses=[r['name'] for r in rs if r['before']['cli_comparison']==0 and r['after']['cli_comparison']!=0],introduced_rejection_losses=[r['name'] for r in rs if not r['observed_valid'] and r['before_cli']!=0 and r['after_cli']==0],new_valid_errors=[r['name'] for r in rs if r['observed_valid'] and r['before_cli']==0 and r['after_cli']!=0],after_svg_panics=[r['name'] for r in rs if r['after_svg_panic']],after_yaml_panics=[r['name'] for r in rs if r['after_yaml_exit']==101],positive_recoveries=sum(r['before']['cli_comparison'] not in [0,2] and r['after']['cli_comparison']==0 for r in rs))
old=read(C/'packet2371-after/measurements.json');oldhashes={r['source_sha256'] for r in old};summary=dict(total=stats(rows),mechanisms={m:stats([r for r in rows if r['mechanism']==m]) for m in sorted({r['mechanism'] for r in rows})},freshness=dict(entries=len(rows),unique_sources=len({r['source_sha256'] for r in rows}),overlap_with_packet2371=[r['name'] for r in rows if r['source_sha256'] in oldhashes]),note_heights=dict(entries=len(geometry),after_height_equal=sum(r['height_equal_after'] for r in geometry),height_failures=[r['name'] for r in geometry if not r['height_equal_after']]),rejection_contract='Four new false accepts are introduced lost passes, separate from zero positive-SVG pass losses. Invalid references never earn positive parity credit.')
for name,value in [('fresh-results.json',rows),('fresh-summary.json',summary),('note-geometry.json',geometry),('note-ownership.json',ownership),('participant-display.json',display)]: (C/name).write_text(json.dumps(value,indent=2)+'\n')
print(json.dumps(summary,indent=2));print('boundary geometry',[(r['name'],r['max_paint_height']) for r in geometry if '512' in r['name'] or '557' in r['name']])
