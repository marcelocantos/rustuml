# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,math,re,xml.etree.ElementTree as ET
from pathlib import Path
s=Path(__file__).resolve().parent
records=[]
for case in json.loads((s/'comparison.json').read_text())['results']:
 name=case['name'];p=s/'pairs'/name
 if not any(v in name for v in ['boundary_','header_0','header_4','modifier_']):continue
 a=ET.parse(p.with_suffix('.svg')).getroot();b=ET.parse(p.with_suffix('.rust.svg')).getroot()
 item={'name':name,'comparison_status':case['status'],'java_viewBox':a.get('viewBox'),'rust_viewBox':b.get('viewBox')}
 if 'boundary_' in name:
  label=next(e for e in a.iter() if e.tag.endswith('}text') and e.text=='Service gate')
  item.update(java_label_baseline=float(label.get('y')),rust_label_baseline=float(next(e for e in b.iter() if e.tag.endswith('}text') and e.text=='Service gate').get('y')))
  # LimitFinder.drawText max = baseline + 1.5; SvekResult adds 15.
  item['java_canvas_height_from_painted_text']=math.floor(item['java_label_baseline']+1.5)+15
  assert item['java_canvas_height_from_painted_text']==int(a.get('height').removesuffix('px'))
 if 'header_' in name:
  def spot(root):
   g=next(e for e in root.iter() if e.get('class')=='entity');return next(e for e in g if e.tag.endswith('}ellipse')),next(e for e in g if e.tag.endswith('}path'))
  ae,ap=spot(a);be,bp=spot(b)
  item.update(java_circle=ae.attrib,rust_circle=be.attrib,java_path_commands=''.join(re.findall('[A-Za-z]',ap.get('d'))),rust_path_commands=''.join(re.findall('[A-Za-z]',bp.get('d'))))
  assert ae.attrib==be.attrib
 if 'modifier_' in name:
  def lineage(root):
   ans=[]
   def walk(e,path):
    here=path+[e.tag.rsplit('}',1)[-1]]
    if e.tag.endswith('}a'):ans.append({'path':here,'attributes':e.attrib})
    for child in e:walk(child,here)
   walk(root,[]);return ans
  item.update(java_anchors=lineage(a),rust_anchors=lineage(b))
 records.append(item)
(s/'observations.json').write_text(json.dumps(records,indent=2)+'\n')
print('Preserved bounded XML observations for',len(records),'controls; boundary painted-text canvas derivation and four circle attribute equalities verified.')
