# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent source-only replay; retained membership, fresh before AND after runs."""
import concurrent.futures,gzip,hashlib,json,os,subprocess,sys,xml.etree.ElementTree as ET
from pathlib import Path
import yaml
R=Path.cwd();S=Path('/tmp/rustuml-checker6');P=R/'docs/parity-reviews/tim-builtin-value-boundaries/checker5-support'
def load(p):return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
def sha(b):return hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
def remap(p):
 s=str(p)
 for old in ['/Users/marcelo/.t3/worktrees/rustuml/t3-04b6729a','/Users/marcelo/.t3/worktrees/rustuml/t3-2c674563']:s=s.replace(old,str(R))
 return R/Path(s)
def model(b):
 root={k.value:v for k,v in yaml.compose(b).value};d={k.value:v for k,v in root['diagram'].value};meta={k.value:v for k,v in d['meta'].value};return dict(type=root['type'].value,entities=len(d['entities'].value) if 'entities' in d else None,source=meta['source'].value)
def run(cmd,cwd,env,data=None):
 try:return subprocess.run(list(map(str,cmd)),input=data,cwd=cwd,env=env,capture_output=True,timeout=90)
 except subprocess.TimeoutExpired as e:return subprocess.CompletedProcess(cmd,124,e.stdout or b'',(e.stderr or b'')+b'\nCHECKER_TIMEOUT')
def measure(item):
 suite,index,original=item;row={k:v for k,v in original.items() if k not in ['phases','before','current','measurements']}
 p=remap(row['source']);gold=remap(row.get('golden',p.with_suffix('.svg')));data=p.read_bytes();gb=gold.read_bytes()
 for field,actual in [('source_sha256',sha(data)),('golden_sha256',sha(gb))]:
  if field in row:assert row[field]==actual,(p,field)
  row[field]=actual
 try:ET.fromstring(gb);xml=True
 except ET.ParseError:xml=False
 ident=f'{index:04d}-'+p.stem;d=S/'replay'/suite/ident;d.mkdir(parents=True,exist_ok=True)
 env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG=row.get('debug','date=1774210426000,tz=AEDT+1100'),TZ=row.get('zone','GMT+11:00'),**row.get('env',{}));cwd=remap(row.get('cwd','.'))
 refsource=remap(row['reference_source']).read_bytes() if 'reference_source' in row else None
 phases={}
 for phase in ['before','after']:
  dest=d/phase;dest.mkdir(exist_ok=True);calls=[]
  for mode,argv in [('svg',[S/(phase+'-rustuml'),'-pipe']),('yaml',[S/(phase+'-rustuml'),'--yaml','-pipe'])]:
   q=run(argv,cwd,env,data);(dest/('rust.'+mode)).write_bytes(q.stdout);(dest/(mode+'.stderr')).write_bytes(q.stderr)
   calls.append(dict(mode=mode,argv=list(map(str,argv)),cwd=str(cwd),env_overrides={k:env[k] for k in ['TZ','RUSTUML_DEBUG','PLANTUML_STDLIB','RAYON_NUM_THREADS']}|row.get('env',{}),exit=q.returncode,stdout_sha256=sha(q.stdout),stderr_sha256=sha(q.stderr)))
  cmd=[S/'probe','compare',gold,dest/'rust.svg'];q=run(cmd,R,env);(dest/'comparison.stdout').write_bytes(q.stdout);(dest/'comparison.stderr').write_bytes(q.stderr)
  valid=xml and q.returncode!=2
  z=dict(cli_exit=calls[0]['exit'],comparison_exit=q.returncode,svg_match=valid and calls[0]['exit']==0 and q.returncode==0,panic=b'panicked at' in (dest/'svg.stderr').read_bytes(),calls=calls,comparison_argv=list(map(str,cmd)),first_diagnostic=(dest/'svg.stderr').read_text(errors='replace').splitlines()[:4])
  try:
   z['model']=model((dest/'rust.yaml').read_bytes());src=z['model']['source'].encode();(dest/'rust.source').write_bytes(src)
   if refsource is not None:z['source_equal']=valid and src==refsource;z['reference_source_sha256']=sha(refsource)
  except (TypeError,ValueError,KeyError,AttributeError,yaml.YAMLError):z['source_equal']=False
  phases[phase]=z
 if 'reference_valid' in row:assert row['reference_valid']==valid,(p,'reference reclassification')
 result=dict(row,suite=suite,capture_id=ident,reference_valid=valid,reference_xml_valid=xml,phases=phases,actual_source=str(p),actual_golden=str(gold))
 dump(d/'result.json',result);return result
if __name__=='__main__':
 for suite in sys.argv[1:]:
  if suite=='maker18':
   root=R/'docs/parity-reviews/tim-rpn-call-state/maker-support';rows=load(root/'relay18/results.json')
   for r in rows:
    stem=Path(r['source']).stem;r.update(source=str(root/'inputs'/f'{stem}.puml'),golden=str(root/'inputs'/f'{stem}.svg'),reference_source=str(root/'relay18'/stem/'java.source.stdout'),cwd=str(root/'inputs'))
  else:rows=load(P/('fresh-results.json' if suite=='fresh29' else suite+'-results.json.gz'))
  with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:out=list(pool.map(measure,[(suite,i,r) for i,r in enumerate(rows)]))
  dump(S/(suite+'-results.json'),out)
  summary=dict(total=len(out),valid=sum(r['reference_valid'] for r in out),before=sum(r['phases']['before']['svg_match'] for r in out),after=sum(r['phases']['after']['svg_match'] for r in out),source_matches=sum(r['phases']['after'].get('source_equal',False) for r in out),negative_rejected=sum(not r['reference_valid'] and r['phases']['after']['cli_exit']!=0 for r in out),panics=sum(r['phases']['after']['panic'] for r in out),losses=[r['source'] for r in out if r['phases']['before']['svg_match'] and not r['phases']['after']['svg_match']],new_valid_errors=[r['source'] for r in out if r['reference_valid'] and not r['phases']['before']['cli_exit'] and r['phases']['after']['cli_exit']],valid_model_drops=[r['source'] for r in out if r['reference_valid'] and r['phases']['before'].get('model',{}).get('entities') is not None and (r['phases']['after'].get('model',{}).get('entities') or 0)<r['phases']['before']['model']['entities']])
  dump(S/(suite+'-summary.json'),summary);print(suite,json.dumps(summary),flush=True)
