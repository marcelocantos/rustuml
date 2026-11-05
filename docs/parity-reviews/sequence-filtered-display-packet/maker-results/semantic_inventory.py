# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Retain structural counts independently of whole-case comparator status."""
import collections,json,re,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd()
def read(p):return json.loads(p.read_text())
def shape(path):
 try:root=ET.parse(path).getroot()
 except (ET.ParseError,FileNotFoundError):return {'xml':False}
 tags=collections.Counter();texts=collections.Counter();heads=collections.Counter();geometry=[]
 for e in root.iter():
  tag=e.tag.rsplit('}',1)[-1];tags[tag]+=1
  if tag=='text':texts[''.join(e.itertext())]+=1
  if 'participant-head' in e.attrib.get('class',''):heads[e.attrib.get('data-qualified-name',e.attrib.get('id',''))]+=1
  if tag in ['ellipse','circle','rect','path','polygon','line']:geometry.append(dict(tag=tag,attributes=e.attrib))
 return dict(xml=True,tags=dict(tags),texts=dict(texts),head_groups=dict(heads),geometry=geometry)
before=[]
for folder in ['before2345','before26']:
 before.extend((r,C/folder) for r in read(C/folder/'measurements.json'))
after=read(C/'final2371/measurements.json');assert len(before)==len(after)==2371
rows=[];worse=[]
for (a,folder),b in zip(before,after):
 assert a['source']==b['source']
 shapes=dict(java=shape(R/b['golden']),before=shape(folder/a['cli']['svg']),after=shape(C/'final2371'/b['cli']['svg']))
 def excess(which,key):
  return sum((collections.Counter(shapes[which].get(key,{}))-collections.Counter(shapes['java'].get(key,{}))).values())
 changed=shapes['before']!=shapes['after'];counts_changed=any(shapes['before'].get(k)!=shapes['after'].get(k) for k in ['tags','texts','head_groups'])
 row=dict(source=b['source'],comparator_before=a['cli_comparison']['exit'],comparator_after=b['cli_comparison']['exit'],changed=changed,counts_changed=counts_changed,shapes=shapes,excess={k:{w:excess(w,k) for w in ['before','after']} for k in ['tags','texts']})
 rows.append(row)
 if b['cli_comparison']['exit']!=2 and any(x['after']>x['before'] for x in row['excess'].values()):worse.append(row['source'])
(C/'semantic-inventory.json').write_text(json.dumps(rows,indent=2)+'\n')
models=read(C/'final-model-and-ascii.json');counts=[]
for r in models:
 values={}
 for rev in ['before','after']:
  o=r['results'][rev]['model'];txt=o['stdout'];values[rev]=dict(exit=o['exit'],raw_create_count=len(re.findall(r'^\s+Create\(',txt,re.M)),raw_message_count=len(re.findall(r'^\s+Message\(',txt,re.M)),raw_note_count=len(re.findall(r'^\s+Note\(',txt,re.M)),model_sha256=o['stdout_sha256'])
 counts.append(dict(source=r['source'],values=values))
(C/'model-event-counts.json').write_text(json.dumps(counts,indent=2)+'\n')
witnesses=[r for r in rows if any(s in r['source'] for s in ['ind5f_hide_widename','ind5f_exo_note','hide_kinds','hide_created','bottom_order','bottom_created','exo_note','exo_stale','exo_right','rect_rows','hex_rows','flush_eol_nested','hide_closing_capture'])]
(C/'creation-witnesses.json').write_text(json.dumps(witnesses,indent=2)+'\n')
summary=dict(entries=len(rows),changed_geometry=sum(r['changed'] for r in rows),changed_counts=sum(r['counts_changed'] for r in rows),new_or_larger_excess_counts=worse,model_entries=len(models),note='Counts are diagnostics, not semantic completeness or a substitute for whole comparison. Java creation heads are not consistently grouped; raw ellipse/shape and label counts are retained rather than inventing a universal head-group metric.')
(C/'semantic-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
