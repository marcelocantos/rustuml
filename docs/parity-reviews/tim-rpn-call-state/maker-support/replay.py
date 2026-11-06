# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Replay immutable checker5 memberships; no Java or network actions."""
import concurrent.futures,gzip,hashlib,json,os,subprocess,sys,xml.etree.ElementTree as ET
from pathlib import Path
import yaml
R=Path.cwd();S=Path('/tmp/rustuml-tim6');P=R/'docs/parity-reviews/tim-builtin-value-boundaries/checker5-support';oldroot='/Users/marcelo/.t3/worktrees/rustuml/t3-04b6729a'
phase,suite=sys.argv[1:]
def load(p):return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
def sha(b):return hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
def model(b):
 root={k.value:v for k,v in yaml.compose(b).value};d={k.value:v for k,v in root['diagram'].value};meta={k.value:v for k,v in d['meta'].value};return dict(type=root['type'].value,entities=len(d['entities'].value) if 'entities' in d else None,source=meta['source'].value)
def run(cmd,cwd,env,data=None):
 try:return subprocess.run(list(map(str,cmd)),input=data,cwd=cwd,env=env,capture_output=True,timeout=90)
 except subprocess.TimeoutExpired as e:return subprocess.CompletedProcess(cmd,124,e.stdout or b'',(e.stderr or b'')+b'\nMAKER_TIMEOUT')
def remap(p):return Path(str(p).replace(oldroot,str(R)))
file='fresh-results.json' if suite=='fresh29' else suite+'-results.json.gz'
rows=load(P/file)
def measure(original):
 row={k:v for k,v in original.items() if k!='phases'}
 for field in ['source','golden','reference_source','cwd']:
  if field in row:row[field]=str(remap(row[field]))
 p=R/row['source'];gold=Path(row.get('golden',p.with_suffix('.svg')));data=p.read_bytes();gb=gold.read_bytes()
 assert sha(data)==row['source_sha256'],str(p)
 assert sha(gb)==row['golden_sha256'],str(gold)
 ident=original['capture_id'];d=S/'replay'/phase/suite/ident;d.mkdir(parents=True,exist_ok=True)
 env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG=row.get('debug','date=1774210426000,tz=AEDT+1100'),TZ=row.get('zone','GMT+11:00'),**row.get('env',{}));cwd=R/row.get('cwd','.')
 binary=S/(phase+'-rustuml');calls=[]
 # All 1516 also run model capture: retain existing valid-parse status and model drops.
 for mode,argv in [('svg',[binary,'-pipe']),('yaml',[binary,'--yaml','-pipe'])]:
  q=run(argv,cwd,env,data);(d/('rust.'+mode)).write_bytes(q.stdout);(d/(mode+'.stderr')).write_bytes(q.stderr)
  calls.append(dict(mode=mode,argv=list(map(str,argv)),cwd=str(cwd),env_overrides={k:env[k] for k in ['TZ','RUSTUML_DEBUG','PLANTUML_STDLIB','RAYON_NUM_THREADS']}|row.get('env',{}),exit=q.returncode,stdout_sha256=sha(q.stdout),stderr_sha256=sha(q.stderr)))
 cmd=[S/'probe','compare',gold,d/'rust.svg'];q=run(cmd,R,env);(d/'comparison.stdout').write_bytes(q.stdout);(d/'comparison.stderr').write_bytes(q.stderr)
 valid=original['reference_valid']
 z=dict(cli_exit=calls[0]['exit'],comparison_exit=q.returncode,svg_match=valid and calls[0]['exit']==0 and q.returncode==0,panic=b'panicked at' in (d/'svg.stderr').read_bytes(),calls=calls,comparison_argv=list(map(str,cmd)))
 try:
  z['model']=model((d/'rust.yaml').read_bytes());src=z['model']['source'].encode();(d/'rust.source').write_bytes(src)
  if 'reference_source' in row:
   ref=Path(row['reference_source']).read_bytes();z['source_equal']=valid and src==ref;z['reference_source_sha256']=sha(ref)
 except (TypeError,ValueError,KeyError,AttributeError,yaml.YAMLError):z['source_equal']=False
 result=dict(row,before=original['phases']['after'],current=z,before_provenance='checker5 exact 1703a272 candidate phase after; preserved unchanged')
 dump(d/'result.json',result);return result
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:out=list(pool.map(measure,rows))
dump(S/(phase+'-'+suite+'.json'),out)
summary=dict(total=len(out),valid=sum(r['reference_valid'] for r in out),before=sum(r['before']['svg_match'] for r in out),current=sum(r['current']['svg_match'] for r in out),source_matches=sum(r['current'].get('source_equal',False) for r in out),negative_rejected=sum(not r['reference_valid'] and r['current']['cli_exit']!=0 for r in out),panics=sum(r['current']['panic'] for r in out),losses=[r['source'] for r in out if r['before']['svg_match'] and not r['current']['svg_match']],gains=[r['source'] for r in out if not r['before']['svg_match'] and r['current']['svg_match']],new_valid_errors=[r['source'] for r in out if r['reference_valid'] and not r['before']['cli_exit'] and r['current']['cli_exit']],model_drops=[r['source'] for r in out if r['before'].get('model',{}).get('entities') is not None and (r['current'].get('model',{}).get('entities') or 0)<r['before']['model']['entities']])
for cohort in ['cohort955','cohort1044']:summary[cohort]=dict(total=sum(bool(r.get(cohort)) for r in out),current=sum(r['current']['svg_match'] for r in out if r.get(cohort)))
dump(S/(phase+'-'+suite+'-summary.json'),summary);print(suite,json.dumps(summary),flush=True)
if suite=='fresh29':
 for r in out:print(Path(r['source']).stem,r['reference_valid'],r['current']['cli_exit'],r['current']['svg_match'],r['current'].get('source_equal'),r['current'].get('model',{}).get('source',''))
