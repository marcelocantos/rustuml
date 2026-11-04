# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Verify TIM18 remote byte transport and evaluate shipped local CLI paths."""
import base64,gzip,hashlib,json,os,shutil,subprocess,sys,xml.etree.ElementTree as ET
from pathlib import Path
import yaml
S=Path('/tmp/rustuml-tim6');C=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay');phase_name=sys.argv[1] if len(sys.argv)>1 else 'iteration1';O=S/('relay18' if phase_name=='iteration1' else 'relay18-'+phase_name);O.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
blob=(C/'tim18-results.json.gz').read_bytes();assert sha(blob)=='b204b17dee9a3d4ee6ec60b0c67ac835621d9ee8947f04da3db8bdd3a106e943'
raw=gzip.decompress(blob);assert raw==(C/'tim18-results.json').read_bytes();bundle=json.loads(raw);env=bundle['environment']
req=json.loads((S/'remote-request.json').read_bytes());manifest=json.loads((S/'new-manifest.json').read_bytes());expected={r['source']:r for r in manifest}
assert env['request_sha256']==sha((S/'remote-request.json').read_bytes())=='7be8f4bac187e38dee21d8812613dc1ac93f2e6f379da57b738496693d5629a5'
assert env['manifest_sha256']==req['manifest_sha256']==sha((S/'new-manifest.json').read_bytes())=='ebed72f9cb2872e86be271b76f8affaaf376d86ae5a522e50aeee156f2175890'
assert env['jar_sha256']==req['jar_sha256'];assert env['headless'] is True
assert bundle['resources']==req['resources'] and len(bundle['resources'])==19
for r in bundle['resources']:
 b=Path(r['local_path']).read_bytes();assert len(b)==r['bytes'] and sha(b)==r['sha256']
assert len(bundle['results'])==len(expected)==18 and {r['source'] for r in bundle['results']}==set(expected)
contracts={r['source']:r for r in req['case_contracts']};results=[]
for ref in bundle['results']:
 row=expected[ref['source']];p=Path(ref['source']);assert sha(p.read_bytes())==ref['source_sha256'];d=O/p.stem;d.mkdir(exist_ok=True)
 for mode,call in [('svg',ref),('source',ref['probe'])]:
  planned=[x.replace('<worker>',env['worker']) for x in contracts[ref['source']]['svg_argv' if mode=='svg' else 'probe_argv']];planned[0]=req['wrapper'];assert call['argv']==planned
  assert not call['timed_out'];assert call['cwd']==str(Path(env['worker'])/row['cwd']);assert call['env_overrides']==row['env'];assert call['timezone']==row['zone']
  assert '-Djava.awt.headless=true' in call['java_argv'];assert '-Duser.timezone='+row['zone'] in call['java_argv'];assert env['jar'] in ':'.join(call['java_argv'])
  for stream in ['stdout','stderr']:
   b=base64.b64decode(call[stream+'_base64'],validate=True);assert sha(b)==call[stream+'_sha256'];(d/('java.'+mode+'.'+stream)).write_bytes(b)
 java=(d/'java.svg.stdout').read_bytes()
 try:ET.fromstring(java);xml_valid=True
 except ET.ParseError:xml_valid=False
 comparisons={}
 for phase in ['before',phase_name]:
  dest=d/phase;dest.mkdir(exist_ok=True);calls=[]
  localenv=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUSTUML_DEBUG=row['debug'],TZ=row['zone'],PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',**row['env'])
  for mode,args in [('svg',['-pipe']),('yaml',['--yaml','-pipe'])]:
   cmd=[S/(phase+'-rustuml'),*args];q=subprocess.run(cmd,input=p.read_bytes(),cwd=p.parent,env=localenv,capture_output=True,timeout=90)
   (dest/('rust.'+mode)).write_bytes(q.stdout);(dest/(mode+'.stderr')).write_bytes(q.stderr)
   calls.append(dict(argv=list(map(str,cmd)),cwd=str(p.parent),env_overrides={k:localenv[k] for k in ['TZ','RUSTUML_DEBUG','PLANTUML_STDLIB','RAYON_NUM_THREADS']}|row['env'],exit_code=q.returncode,stdout_sha256=sha(q.stdout),stderr_sha256=sha(q.stderr)))
  cmd=[S/'probe','compare',d/'java.svg.stdout',dest/'rust.svg'];q=subprocess.run(cmd,capture_output=True,timeout=90);(dest/'comparison.stdout').write_bytes(q.stdout);(dest/'comparison.stderr').write_bytes(q.stderr)
  valid=ref['exit_code']==0 and xml_valid and q.returncode!=2
  source_equal=False
  try:
   node=yaml.compose((dest/'rust.yaml').read_bytes())
   for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
   value=node.value.encode();(dest/'rust.source').write_bytes(value);source_equal=value==(d/'java.source.stdout').read_bytes()
  except (TypeError,ValueError,KeyError,AttributeError,yaml.YAMLError):pass
  comparisons[phase]=dict(cli_exit=calls[0]['exit_code'],svg_match=valid and calls[0]['exit_code']==0 and q.returncode==0,source_equal=valid and ref['probe']['exit_code']==0 and source_equal,comparison_exit=q.returncode,calls=calls,comparison_argv=list(map(str,cmd)))
 result=dict(row,source_sha256=ref['source_sha256'],java_exit=ref['exit_code'],java_probe_exit=ref['probe']['exit_code'],xml_valid=xml_valid,reference_valid=valid,measurements=comparisons)
 dump(d/'result.json',result);dump(d/'commands.json',{k:v for k,v in ref.items() if 'base64' not in k and k!='probe'}|{'probe':{k:v for k,v in ref['probe'].items() if 'base64' not in k}});results.append(result)
 print(p.stem,'java',ref['exit_code'],'XML',xml_valid,'rust',comparisons[phase_name]['cli_exit'],'svg',comparisons[phase_name]['svg_match'],'source',comparisons[phase_name]['source_equal'])
(O/'tim18-results.json.gz').write_bytes(blob);shutil.copy2(C/'tim18-transport.json',O/'tim18-transport.json');dump(O/'environment.json',env);dump(O/'results.json',results)
summary=dict(total=len(results),valid=sum(r['reference_valid'] for r in results),negative=sum(not r['reference_valid'] for r in results),before_matches=sum(r['measurements']['before']['svg_match'] for r in results),current_matches=sum(r['measurements'][phase_name]['svg_match'] for r in results),current_source_matches=sum(r['measurements'][phase_name]['source_equal'] for r in results),negative_rejected=sum(not r['reference_valid'] and r['measurements'][phase_name]['cli_exit']!=0 for r in results),lost_passes=[r['source'] for r in results if r['measurements']['before']['svg_match'] and not r['measurements'][phase_name]['svg_match']],valid_failures=[r['source'] for r in results if r['reference_valid'] and not r['measurements'][phase_name]['svg_match']],new_valid_errors=[r['source'] for r in results if r['reference_valid'] and r['measurements'][phase_name]['cli_exit'] and not r['measurements']['before']['cli_exit']],input_compressed_sha256=env['input_compressed_sha256'],output_compressed_sha256=sha(blob),output_json_sha256=sha(raw),resources_verified=len(bundle['resources']),calls_verified=36)
dump(O/'summary.json',summary);print(json.dumps(summary,indent=2))
