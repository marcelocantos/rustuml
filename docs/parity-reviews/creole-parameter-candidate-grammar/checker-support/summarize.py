# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,collections,hashlib,yaml
from pathlib import Path
S=Path(__file__).parent;M=S/'maker';cohorts=['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12','checker28','fresh16','golden-regressions','checker36'];summary=[];allmodels=[];repro=[]
def canonical(n):
 if n is None:return None
 if isinstance(n,yaml.MappingNode):return (n.tag,sorted((canonical(k),canonical(v)) for k,v in n.value))
 if isinstance(n,yaml.SequenceNode):return (n.tag,[canonical(v) for v in n.value])
 return (n.tag,n.value)
for c in cohorts:
 b=json.loads((S/(c+'-before.json')).read_text());a=json.loads((S/(c+'-after.json')).read_text());assert len(a)==len(b)
 row=dict(cohort=c,entries=len(a),valid=sum(v['reference_valid'] for v in a),before=sum(v['svg_match'] for v in b),after=sum(v['svg_match'] for v in a),lost=[x['id'] for x,y in zip(b,a) if x['svg_match'] and not y['svg_match']],gained=[x['id'] for x,y in zip(b,a) if not x['svg_match'] and y['svg_match']],new_errors=[x['id'] for x,y in zip(b,a) if x['svg']['exit']==0 and y['svg']['exit']!=0],new_panics=[x['id'] for x,y in zip(b,a) if not x['panic'] and y['panic']],panics_before=sum(v['panic'] for v in b),panics_after=sum(v['panic'] for v in a),svg_exits_before=dict(collections.Counter(v['svg']['exit'] for v in b)),svg_exits_after=dict(collections.Counter(v['svg']['exit'] for v in a)),yaml_exits_before=dict(collections.Counter(v['yaml']['exit'] for v in b)),yaml_exits_after=dict(collections.Counter(v['yaml']['exit'] for v in a)))
 models=[]
 for x,y in zip(b,a):
  bid=S/'replay/before'/x['id'];aid=S/'replay/after'/y['id'];bb=(bid/'yaml.stdout').read_bytes();ab=(aid/'yaml.stdout').read_bytes();semantic=canonical(yaml.compose(bb))==canonical(yaml.compose(ab));models.append(dict(id=x['id'],byte_equal=bb==ab,semantic_equal=semantic));
  for phase,result in [('before',x),('after',y)]:
   p=M/(c+'-'+phase+'.json')
   if p.exists():
    # Load cohort once below; file comparisons use its archived streams.
    orig=M/'replay'/phase/result['id'];own=S/'replay'/phase/result['id']
    repro.append(dict(id=result['id'],phase=phase,svg_bytes_equal=(orig/'svg.stdout').read_bytes()==(own/'svg.stdout').read_bytes(),svg_exit_equal=json.loads((orig/'result.json').read_text())['svg']['exit']==result['svg']['exit'],yaml_semantic_equal=canonical(yaml.compose((orig/'yaml.stdout').read_bytes()))==canonical(yaml.compose((own/'yaml.stdout').read_bytes()))))
 row.update(yaml_byte_equal=sum(x['byte_equal'] for x in models),yaml_semantic_equal=sum(x['semantic_equal'] for x in models),yaml_differences=[x for x in models if not x['byte_equal']]);summary.append(row);allmodels+=models
(S/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');(S/'model-checks.json').write_text(json.dumps(allmodels,indent=2)+'\n');(S/'maker-reproduction.json').write_text(json.dumps(repro,indent=2)+'\n');print(json.dumps(summary,indent=2))
