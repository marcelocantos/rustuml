# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,json,re,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).resolve().parent
out=[]
def inspect(data,names):
 try:root=ET.fromstring(data)
 except ET.ParseError as e:return dict(xml_error=str(e))
 labels=collections.Counter();lines=[];groups=[];shapes=collections.Counter();bars=[];texts=[]
 for e in root.iter():
  tag=e.tag.split('}')[-1];shapes[tag]+=1
  if tag=='text':
   text=''.join(e.itertext());labels[text]+=1;texts.append(dict(text=text,attributes=e.attrib))
  if tag=='line' and e.get('x1')==e.get('x2') and 'stroke-dasharray:5,5' in e.get('style',''):lines.append(e.attrib)
  if tag=='g' and any('participant' in str(v) for v in e.attrib.values()):groups.append(e.attrib)
  if tag=='rect' and e.get('width')=='10':bars.append(e.attrib)
 return dict(labels=dict(labels),participant_head_and_tail_label_counts={name:labels[name] for name in names},head_count_note='Counts include initial, inline and tail heads via unique participant-label occurrences; all labels and shapes retained. Group metadata alone omits ungrouped/Teoz heads. Multiline labels require per-stripe interpretation.',participant_groups=groups,lifeline_segments=len(lines),lifeline_geometry=lines,activation_rectangles=bars,shape_counts=dict(shapes),text_attributes=texts)
for cohort in ['fresh60','followup4']:
 rows=json.loads(gzip.decompress((C/cohort/'measurements.json.gz').read_bytes()))
 for r in rows:
  ast=(C/cohort/r['results']['after']['ast']['output']).read_text();participants=ast.split('        events: [')[0];names=[json.loads(x) for x in re.findall(r'^                label: (".*"),$',participants,re.M)]
  row=dict(name=r['name'],mechanism=r['mechanism'],cohort=cohort,source=r['source'],java=inspect(Path(r['golden']).read_bytes(),names),results={})
  for rev in ['before','after']:
   ast=(C/cohort/r['results'][rev]['ast']['output']).read_text();events=ast.split('        events: [',1)[-1].split('        ],',1)[0] if '        events: [' in ast else ''
   row['results'][rev]=dict(events=dict(collections.Counter(re.findall(r'^            (\w+)(?:\(|,)',events,re.M))),ast_exit=r['results'][rev]['ast']['exit'],svg=inspect((C/cohort/r['results'][rev]['cli']['output']).read_bytes(),names))
  out.append(row)
(C/'semantic-witnesses.json.gz').write_bytes(gzip.compress((json.dumps(out,indent=2)+'\n').encode(),mtime=0))
for name in ['ind5f_exo_repeat_teoz','ind5f_exo_then_other_teoz','ind5f_exo_note_legacy','ind5f_hide_widename_created_teoz']:
 r=next(x for x in out if x['name']==name);print(name,'events',r['results']['after']['events'],'heads Java',r['java'].get('participant_head_and_tail_label_counts'),'after',r['results']['after']['svg'].get('participant_head_and_tail_label_counts'),'lifelines',r['java'].get('lifeline_segments'),r['results']['after']['svg'].get('lifeline_segments'))
