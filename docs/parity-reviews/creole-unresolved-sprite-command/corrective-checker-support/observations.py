# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,collections,xml.etree.ElementTree as ET
from pathlib import Path
import yaml
S=Path(__file__).parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
def canonical(n):
 if n is None:return None
 if isinstance(n,yaml.ScalarNode):return [n.tag,n.value]
 if isinstance(n,yaml.SequenceNode):return [n.tag,[canonical(v) for v in n.value]]
 return [n.tag,sorted([[canonical(k),canonical(v)] for k,v in n.value],key=lambda x:json.dumps(x[0],sort_keys=True))]
def svg(p):
 try:
  root=ET.fromstring(p.read_bytes());return dict(canvas=root.attrib,text=[dict(text=''.join(e.itertext()),attrs=e.attrib) for e in root.iter() if e.tag.endswith('}text')],rects=[e.attrib for e in root.iter() if e.tag.endswith('}rect')])
 except ET.ParseError:return None
summary={};obs=[];models=[]
for c in ['retained','maker18','sprites50','checker38','fresh15','fresh26']:
 if not (S/(c+'-after.json')).exists():continue
 rows=json.loads((S/(c+'-membership.json')).read_bytes());before=json.loads((S/(c+'-before.json')).read_bytes());after=json.loads((S/(c+'-after.json')).read_bytes());assert len(rows)==len(before)==len(after)
 pairs=list(zip(rows,before,after));model_changes=[]
 for r,b,a in pairs:
  assert r['id']==b['id']==a['id'];p=S/'replay/before'/r['id'];q=S/'replay/after'/r['id']
  trees=[]
  for d in [p,q]:
   try:trees.append(canonical(yaml.compose((d/'yaml.stdout').read_bytes())))
   except yaml.YAMLError:trees.append(None)
  if trees[0]!=trees[1]:model_changes.append(r['id'])
  if c!='retained' or any(k in r['id'] for k in ['markup_literal','markup_scan','source_line0']):
   obs.append(dict(id=r['id'],source=Path(r['source_capture']).read_text(),before=svg(p/'svg.stdout'),after=svg(q/'svg.stdout'),java=svg(Path(r['golden_capture']))))
   models.append(dict(id=r['id'],before=b['model'],after=a['model'],full_yaml_tree_equal=trees[0]==trees[1]))
 v=dict(entries=len(rows),distinct_pairs=len({(r['source_sha256'],r['golden_sha256']) for r in rows}),valid=sum(b['reference_valid'] for b in before),before=sum(b['svg_match'] for b in before),after=sum(a['svg_match'] for a in after),gains=[b['id'] for r,b,a in pairs if not b['svg_match'] and a['svg_match']],losses=[b['id'] for r,b,a in pairs if b['svg_match'] and not a['svg_match']],failures=[a['id'] for a in after if a['reference_valid'] and not a['svg_match']],nonpositive=[a['id'] for a in after if not a['reference_valid']],panics_before=[b['id'] for b in before if b['panic']],panics_after=[a['id'] for a in after if a['panic']],full_model_changes=model_changes,changed_yaml=[b['id'] for r,b,a in pairs if b['yaml']['stdout_sha256']!=a['yaml']['stdout_sha256']],changed_svg=[b['id'] for r,b,a in pairs if b['svg']['stdout_sha256']!=a['svg']['stdout_sha256']],exact_source_available=sum('exact_source' in a and a['reference_valid'] for a in after),exact_source_equal=sum(a.get('exact_source',False) and a['reference_valid'] for a in after),exits={phase:{mode:dict(collections.Counter(str(x[mode]['exit']) for x in rs)) for mode in ['svg','yaml','compare']} for phase,rs in [('before',before),('after',after)]})
 summary[c]=v;print(c,{k:v[k] for k in ['entries','valid','before','after','losses','full_model_changes','changed_yaml']},flush=True)
dump('summary.json',summary);dump('text-observations.json',obs);dump('model-observations.json',models)
