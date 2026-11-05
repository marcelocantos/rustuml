# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Verify remote TIM15 streams; compare previously captured local CLI outputs."""
import base64,gzip,hashlib,json,shutil,subprocess,xml.etree.ElementTree as ET
from pathlib import Path
import yaml
S=Path('/tmp/rustuml-tim5');C=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay');O=S/'relay15';O.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
blob=(C/'tim15-results.json.gz').read_bytes();assert sha(blob)=='3edc36d366ff72e7b2838963926af9d2d7d11b638bb98263e1cd2aec9f7f1c33'
raw=gzip.decompress(blob);assert raw==(C/'tim15-results.json').read_bytes();bundle=json.loads(raw);env=bundle['environment']
req=json.loads((S/'remote-request.json').read_bytes());manifest=json.loads((S/'new-manifest.json').read_bytes());expected={r['source']:r for r in manifest}
assert env['request_sha256']==sha((S/'remote-request.json').read_bytes())=='7eb6f7446148b437cf104be02461a2fd724cf6ed4a554b7205161d3fd89dc462'
assert env['manifest_sha256']==req['manifest_sha256']==sha((S/'new-manifest.json').read_bytes())=='46deda6a3749d4f7af193438c7920db3ea9aa35e384f39092d2539db41caaf91'
assert env['jar_sha256']==req['jar_sha256'];assert env['headless'] is True
assert bundle['resources']==req['resources'] and len(bundle['resources'])==18
for r in bundle['resources']:
 b=Path(r['local_path']).read_bytes();assert len(b)==r['bytes'] and sha(b)==r['sha256']
assert len(bundle['results'])==len(expected)==15 and {r['source'] for r in bundle['results']}==set(expected)
contracts={r['source']:r for r in req['case_contracts']}
local={(r['phase'],r['source']):r for r in json.loads((S/'new-rust-results.json').read_bytes())};results=[]
for ref in bundle['results']:
 row=expected[ref['source']];p=Path(ref['source']);assert sha(p.read_bytes())==ref['source_sha256'];d=O/p.stem;d.mkdir(exist_ok=True)
 for mode,call in [('svg',ref),('source',ref['probe'])]:
  planned=[x.replace('<worker>',env['worker']) for x in contracts[ref['source']]['svg_argv' if mode=='svg' else 'probe_argv']];planned[0]=req['wrapper'];assert call['argv']==planned
  assert not call['timed_out'];assert call['cwd']==str(Path(env['worker'])/row['cwd']);assert call['env_overrides']==row['env'];assert call['timezone']==row['zone']
  assert call['argv'][0]==req['wrapper'];assert call['argv'][call['argv'].index('--timezone')+1]==row['zone']
  assert '-Djava.awt.headless=true' in call['java_argv'];assert '-Duser.timezone='+row['zone'] in call['java_argv'];assert env['jar'] in ':'.join(call['java_argv'])
  for stream in ['stdout','stderr']:
   b=base64.b64decode(call[stream+'_base64'],validate=True);assert sha(b)==call[stream+'_sha256'];(d/('java.'+mode+'.'+stream)).write_bytes(b)
 java=(d/'java.svg.stdout').read_bytes()
 try:ET.fromstring(java);xml_valid=True
 except ET.ParseError:xml_valid=False
 # Preserve source probe status independently; it can succeed on an ERROR SVG.
 comparisons={}
 for phase in ['before','iteration1']:
  src=S/'new-rust'/phase/p.stem;dest=d/phase;dest.mkdir(exist_ok=True)
  for f in src.iterdir():shutil.copy2(f,dest/f.name)
  cmd=[S/'probe','compare',d/'java.svg.stdout',dest/'rust.svg'];q=subprocess.run(cmd,capture_output=True,timeout=90);(dest/'comparison.txt').write_bytes(q.stdout+q.stderr)
  valid=ref['exit_code']==0 and xml_valid and q.returncode!=2
  source_equal=False
  try:
   node=yaml.compose((dest/'rust.yaml').read_bytes())
   for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
   value=node.value.encode();(dest/'rust.source').write_bytes(value);source_equal=value==(d/'java.source.stdout').read_bytes()
  except (TypeError,ValueError,KeyError,AttributeError,yaml.YAMLError):pass
  calls=local[(phase,str(p))]['calls'];comparisons[phase]=dict(cli_exit=calls[0]['exit_code'],svg_match=valid and calls[0]['exit_code']==0 and q.returncode==0,source_equal=valid and ref['probe']['exit_code']==0 and source_equal,comparison_exit=q.returncode,calls=calls,comparison_argv=list(map(str,cmd)))
 result=dict(row,source_sha256=ref['source_sha256'],java_exit=ref['exit_code'],java_probe_exit=ref['probe']['exit_code'],xml_valid=xml_valid,reference_valid=valid,measurements=comparisons)
 dump(d/'result.json',result);dump(d/'commands.json',{k:v for k,v in ref.items() if 'base64' not in k and k!='probe'}|{'probe':{k:v for k,v in ref['probe'].items() if 'base64' not in k}});results.append(result)
 print(p.stem,'java',ref['exit_code'],'XML',xml_valid,'rust',comparisons['iteration1']['cli_exit'],'svg',comparisons['iteration1']['svg_match'],'source',comparisons['iteration1']['source_equal'])
(O/'tim15-results.json.gz').write_bytes(blob);shutil.copy2(C/'tim15-transport.json',O/'tim15-transport.json');dump(O/'environment.json',env);dump(O/'results.json',results)
summary=dict(total=len(results),valid=sum(r['reference_valid'] for r in results),negative=sum(not r['reference_valid'] for r in results),before_matches=sum(r['measurements']['before']['svg_match'] for r in results),current_matches=sum(r['measurements']['iteration1']['svg_match'] for r in results),current_source_matches=sum(r['measurements']['iteration1']['source_equal'] for r in results),negative_rejected=sum(not r['reference_valid'] and r['measurements']['iteration1']['cli_exit']!=0 for r in results),lost_passes=[r['source'] for r in results if r['measurements']['before']['svg_match'] and not r['measurements']['iteration1']['svg_match']],valid_failures=[r['source'] for r in results if r['reference_valid'] and not r['measurements']['iteration1']['svg_match']],new_valid_errors=[r['source'] for r in results if r['reference_valid'] and r['measurements']['iteration1']['cli_exit'] and not r['measurements']['before']['cli_exit']],input_compressed_sha256=env['input_compressed_sha256'],output_compressed_sha256=sha(blob),output_json_sha256=sha(raw),resources_verified=len(bundle['resources']),calls_verified=30)
dump(O/'summary.json',summary);print(json.dumps(summary,indent=2))
