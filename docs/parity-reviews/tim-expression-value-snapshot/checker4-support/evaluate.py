# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Verify the immutable byte relay, then measure the real CLI, never local Java."""
import base64,concurrent.futures,gzip,hashlib,json,os,shutil,subprocess,sys,xml.etree.ElementTree as ET
from pathlib import Path
import yaml
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-cb14b5f3');S=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3');T=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay')
sha=lambda b:hashlib.sha256(b).hexdigest()
req=json.loads((S/'remote-request.json').read_bytes());rows=json.loads((S/'fresh-manifest.json').read_bytes())
assert sha((S/'fresh-manifest.json').read_bytes())==req['manifest_sha256']
z=(T/'tim29-results.json.gz').read_bytes();assert sha(z)=='1562e919a0c83969fa7cd3b27ca68a57d15b7ba1a892b87d00f5f8b31891758e'
raw=gzip.decompress(z);assert raw==(T/'tim29-results.json').read_bytes();bundle=json.loads(raw)
assert bundle['environment']['jar_sha256']==req['jar_sha256']
assert bundle['environment']['manifest_sha256']==req['manifest_sha256']
assert bundle['environment']['input_compressed_sha256']=='d1e86a9454df102d8d54ae14c1a37cd3e74d57a2348569d2e7d9ed027bfc9c07'
resources={r['remote_relative_path']:r for r in req['resources']};returned={r['remote_relative_path']:r for r in bundle['resources']}
assert resources==returned and len(resources)==34
for rel,entry in resources.items():
 data=Path(entry['local_path']).read_bytes();assert len(data)==entry['bytes'] and sha(data)==entry['sha256']
 if rel.startswith('test-diagrams/'):
  p=R/rel;p.parent.mkdir(parents=True,exist_ok=True)
  if p.exists():assert p.read_bytes()==data
  else:p.write_bytes(data)
refs={r['source']:r for r in bundle['results']};assert set(refs)=={r['source'] for r in rows} and len(refs)==29
for row in rows:
 ref=refs[row['source']];assert ref['source_sha256']==row['source_sha256']==sha((R/row['source']).read_bytes())
 d=S/'fresh-reference'/row['id'];d.mkdir(parents=True,exist_ok=True)
 for mode,call in [('svg',ref),('source',ref['probe'])]:
  assert call['env_overrides']==row['env'] and call['argv'][call['argv'].index('--timezone')+1]==row['zone']
  assert Path(call['cwd'])==Path(bundle['environment']['worker'])/row['cwd']
  assert not call['timed_out']
  for stream in ['stdout','stderr']:
   data=base64.b64decode(call[stream+'_base64'],validate=True);assert sha(data)==call[stream+'_sha256']
   (d/(mode+'.'+stream)).write_bytes(data)
 (R/row['source']).with_suffix('.svg').write_bytes((d/'svg.stdout').read_bytes())
 (d/'commands.json').write_text(json.dumps({k:v for k,v in ref.items() if k not in ['stdout_base64','stderr_base64','probe']}|{'probe':{k:v for k,v in ref['probe'].items() if 'base64' not in k}},indent=2)+'\n')
(S/'relay-verification.json').write_text(json.dumps(dict(input_compressed_sha256=bundle['environment']['input_compressed_sha256'],output_compressed_sha256=sha(z),output_json_sha256=sha(raw),manifest_sha256=req['manifest_sha256'],resources_verified=len(resources),cases_verified=len(rows),environment=bundle['environment']),indent=2)+'\n')
phase=sys.argv[1]
binary=S/('candidate-rustuml' if phase=='candidate' else 'baseline-target/release/rustuml')
def run(cmd,source,cwd,env):
 try:return subprocess.run(list(map(str,cmd)),input=source,cwd=cwd,env=env,capture_output=True,timeout=90)
 except subprocess.TimeoutExpired as e:return subprocess.CompletedProcess(cmd,124,e.stdout or b'',(e.stderr or b'')+b'CHECKER_TIMEOUT')
def measure(row):
 p=R/row['source'];ref=refs[row['source']];d=S/'fresh-outputs'/phase/row['id'];d.mkdir(parents=True,exist_ok=True)
 env=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG=row['debug'],TZ=row['zone'],RAYON_NUM_THREADS='4',**row['env'])
 commands=[]
 for mode,args in [('svg',['-pipe']),('yaml',['--yaml','-pipe'])]:
  cmd=[binary,*args];q=run(cmd,p.read_bytes(),R/row['cwd'],env)
  (d/('rust.'+mode)).write_bytes(q.stdout);(d/(mode+'.stderr')).write_bytes(q.stderr);commands.append(dict(argv=list(map(str,cmd)),cwd=str(R/row['cwd']),env_overrides=dict(row['env'],TZ=row['zone'],RUSTUML_DEBUG=row['debug']),exit=q.returncode))
 cmd=[S/'probe','compare',p.with_suffix('.svg'),d/'rust.svg'];q=run(cmd,None,R,env);(d/'comparison.txt').write_bytes(q.stdout+q.stderr)
 try:ET.fromstring(p.with_suffix('.svg').read_bytes());malformed=False
 except ET.ParseError:malformed=True
 valid=ref['exit_code']==0 and q.returncode!=2 and not malformed
 result=dict(row,java_exit=ref['exit_code'],java_probe_exit=ref['probe']['exit_code'],cli_exit=commands[0]['exit'],comparison_exit=q.returncode,reference_valid=valid,svg_match=valid and commands[0]['exit']==0 and q.returncode==0,panic=b'panicked at' in (d/'svg.stderr').read_bytes(),commands=commands,comparison_command=list(map(str,cmd)))
 try:
  node=yaml.compose((d/'rust.yaml').read_bytes())
  for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
  rawsource=node.value.encode();(d/'rust.source').write_bytes(rawsource)
  result['source_equal']=valid and ref['probe']['exit_code']==0 and rawsource==(S/'fresh-reference'/row['id']/'source.stdout').read_bytes()
 except (ValueError,TypeError,KeyError,AttributeError,yaml.YAMLError):result['source_equal']=False
 (d/'result.json').write_text(json.dumps(result,indent=2)+'\n');return result
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(measure,rows))
(S/f'fresh-{phase}.json').write_text(json.dumps(results,indent=2)+'\n')
for r in results:print(r['id'],'java',r['java_exit'],'rust',r['cli_exit'],'SVG',r['svg_match'],'source',r['source_equal'])
print('total',len(results),'valid',sum(r['reference_valid'] for r in results),'matches',sum(r['svg_match'] for r in results),'sources',sum(r['source_equal'] for r in results))
