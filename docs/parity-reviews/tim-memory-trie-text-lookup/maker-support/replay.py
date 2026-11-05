# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent two-binary replay of frozen membership; never calls Java."""
import concurrent.futures, hashlib, json, os, re, shutil, subprocess, sys, time
from pathlib import Path
import yaml
R=Path.cwd(); S=Path('/Users/marcelo/Library/Caches/rustuml-tim8-maker'); P=Path('/Users/marcelo/Library/Caches/rustuml-tim7-checker'); phase_name,suite=sys.argv[1:]
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,x):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(x,indent=2)+'\n')
def resolve(p):
 p=re.sub(r'/Users/marcelo/\.t3/worktrees/rustuml/t3-[^/]+',str(R),str(p));return R/Path(p)
def model(data):
 node=yaml.compose(data);root={k.value:v for k,v in node.value}; d={k.value:v for k,v in root['diagram'].value};meta={k.value:v for k,v in d['meta'].value}
 return dict(type=root['type'].value,entities=len(d['entities'].value) if 'entities' in d else None,source=meta['source'].value)
def call(argv,cwd,env,data,d,prefix):
 start=time.monotonic()
 try:q=subprocess.run(list(map(str,argv)),input=data,cwd=cwd,env=env,capture_output=True,timeout=90);code=q.returncode;out=q.stdout;err=q.stderr
 except subprocess.TimeoutExpired as e:code=124;out=e.stdout or b'';err=e.stderr or b''
 (d/(prefix+'.stdout')).write_bytes(out);(d/(prefix+'.stderr')).write_bytes(err)
 return dict(argv=list(map(str,argv)),cwd=str(cwd),exit=code,stdout_sha256=sha(out),stderr_sha256=sha(err),seconds=time.monotonic()-start)
def measure(job):
 suite,index,original=job;row=original['membership'];p=resolve(row['source']);g=resolve(row.get('golden',p.with_suffix('.svg')));data=p.read_bytes();gold=g.read_bytes()
 assert sha(data)==row['source_sha256'],str(p)
 assert sha(gold)==row['golden_sha256'],str(g)
 base=S/'replay'/phase_name/suite/f'{index:04}-{p.stem}';base.mkdir(parents=True,exist_ok=True)
 (base/'input.puml').write_bytes(data);(base/'java.svg').write_bytes(gold)
 ref=None
 if 'reference_source' in row:ref=resolve(row['reference_source']).read_bytes();(base/'java.source').write_bytes(ref)
 env_overrides=dict(RUSTUML_DEBUG=row.get('debug','date=1774210426000,tz=AEDT+1100'),TZ=row.get('zone','GMT+11:00'),RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')|row.get('env',{})
 env=dict(os.environ,**env_overrides);cwd=resolve(row.get('cwd','.'));phases={}
 for phase in [phase_name]:
  d=base/phase;d.mkdir(exist_ok=True);binary=S/(phase+'-rustuml')
  svg=call([binary,'-pipe'],cwd,env,data,d,'svg');ym=call([binary,'--yaml','-pipe'],cwd,env,data,d,'yaml');cmp=call([S/'probe','compare',base/'java.svg',d/'svg.stdout'],R,env,None,d,'compare')
  z=dict(svg=svg,yaml=ym,compare=cmp,svg_match=bool(row['reference_valid'] and svg['exit']==0 and cmp['exit']==0),panic=b'panicked at' in (d/'svg.stderr').read_bytes())
  try:
   z['model']=model((d/'yaml.stdout').read_bytes());src=z['model']['source'].encode();(d/'rust.source').write_bytes(src)
   if ref is not None:z['exact_source_equal']=src==ref
  except (yaml.YAMLError,AttributeError,KeyError,ValueError,TypeError):z['model']=None
  phases[phase]=z
 result=dict(before=original['phases']['after'],before_provenance='checker7 exact2cf6 candidate after phase unchanged',classification=original.get('classification'),suite=suite,index=index,input=str(p),golden=str(g),membership=row,env_overrides=env_overrides,phases=phases)
 dump(base/'result.json',result);return result

def main():
 rows=json.loads((P/(suite+'-results.json')).read_bytes())
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:out=list(pool.map(measure,[(suite,i,r) for i,r in enumerate(rows)]))
 dump(S/(phase_name+'-'+suite+'.json'),out)
 summary=dict(total=len(out),valid=sum(r['membership']['reference_valid'] for r in out),before=sum(r['before']['svg_match'] for r in out))
 for phase in [phase_name]:
  summary['current']=dict(svg=sum(r['phases'][phase]['svg_match'] for r in out),source=sum(r['membership']['reference_valid'] and r['phases'][phase].get('exact_source_equal',False) for r in out),source_oracles=sum(r['membership']['reference_valid'] and 'reference_source' in r['membership'] for r in out),negative_rejected=sum(not r['membership']['reference_valid'] and r['phases'][phase]['svg']['exit']!=0 for r in out),panics=sum(r['phases'][phase]['panic'] for r in out))
 summary['lost_passes']=[r['input'] for r in out if r['before']['svg_match'] and not r['phases'][phase_name]['svg_match']]
 summary['gains']=[r['input'] for r in out if not r['before']['svg_match'] and r['phases'][phase_name]['svg_match']]
 summary['new_valid_errors']=[r['input'] for r in out if r['membership']['reference_valid'] and not r['before']['svg']['exit'] and r['phases'][phase_name]['svg']['exit']]
 summary['model_drops']=[dict(source=r['input'],valid=r['membership']['reference_valid']) for r in out if (r['before'].get('model') or {}).get('entities') is not None and ((r['phases'][phase_name].get('model') or {}).get('entities') or 0)<r['before']['model']['entities']]
 dump(S/(phase_name+'-'+suite+'-summary.json'),summary);print(suite,json.dumps(summary),flush=True)
 if suite=='fresh23':
  for r in out:print(Path(r['input']).stem,r['phases'][phase_name].get('exact_source_equal'),r['phases'][phase_name]['svg_match'],r['phases'][phase_name]['svg']['exit'])
if __name__=='__main__':main()
