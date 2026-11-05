# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,json,shutil
from pathlib import Path
from replay import S,R,sha,dump,measure
C=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay');O=S/'relay25';O.mkdir(exist_ok=True)
blob=(C/'tim25-results.json.gz').read_bytes();assert sha(blob)=='4526badf765755ebfef45aaa59f6eabf7f647500c694ba9c8a0c9e2c06e8f29e'
raw=gzip.decompress(blob);assert raw==(C/'tim25-results.json').read_bytes();bundle=json.loads(raw);env=bundle['environment'];req=json.loads((S/'remote-request.json').read_bytes());manifest=json.loads((S/'new-manifest.json').read_bytes());expected={r['source']:r for r in manifest}
assert env['request_sha256']==sha((S/'remote-request.json').read_bytes())=='414e7b6d4ec350e33f2d9211c5ac8fbaed7d41c0e85b628ac55d1b10989a57a4'
assert env['manifest_sha256']==req['manifest_sha256']==sha((S/'new-manifest.json').read_bytes())=='5d5f22068b58b81ce39209d21990ce6fa934d1020786144d5318f5662a42bf80'
assert env['jar_sha256']==req['jar_sha256'];assert env['headless'] is True
assert bundle['resources']==req['resources'] and len(bundle['resources'])==26
for r in bundle['resources']:
 b=Path(r['local_path']).read_bytes();assert len(b)==r['bytes'] and sha(b)==r['sha256']
assert len(bundle['results'])==len(expected)==25 and {r['source'] for r in bundle['results']}==set(expected)
contracts={r['source']:r for r in req['case_contracts']};results=[]
for index,ref in enumerate(bundle['results']):
 row=expected[ref['source']].copy();p=Path(ref['source']);assert sha(p.read_bytes())==ref['source_sha256'];d=O/p.stem;d.mkdir(exist_ok=True)
 for mode,call in [('svg',ref),('source',ref['probe'])]:
  planned=[x.replace('<worker>',env['worker']) for x in contracts[ref['source']]['svg_argv' if mode=='svg' else 'probe_argv']];planned[0]=req['wrapper'];assert call['argv']==planned
  assert not call['timed_out'];assert call['cwd']==str(Path(env['worker'])/row['cwd']);assert call['env_overrides']==row['env'];assert call['timezone']==row['zone']
  assert '-Djava.awt.headless=true' in call['java_argv'];assert '-Duser.timezone='+row['zone'] in call['java_argv'];assert env['jar'] in ':'.join(call['java_argv'])
  for stream in ['stdout','stderr']:
   b=base64.b64decode(call[stream+'_base64'],validate=True);assert sha(b)==call[stream+'_sha256'];(d/('java.'+mode+'.'+stream)).write_bytes(b)
 row.update(golden=str(d/'java.svg.stdout'),reference_source=str(d/'java.source.stdout'),cwd=str(p.parent),java_exit=ref['exit_code'],java_probe_exit=ref['probe']['exit_code'])
 z=measure(('fresh25',index,row));assert not z['reference_valid'] or ref['exit_code']==0
 dump(d/'commands.json',{k:v for k,v in ref.items() if 'base64' not in k and k!='probe'}|{'probe':{k:v for k,v in ref['probe'].items() if 'base64' not in k}});results.append(z)
 print(p.stem,'java',ref['exit_code'],'valid',z['reference_valid'],'rust',z['phases']['after']['cli_exit'],'svg',z['phases']['after']['svg_match'],'source',z['phases']['after'].get('source_equal'))
 if not z['phases']['after'].get('source_equal'):
  print('  java-source',repr((d/'java.source.stdout').read_text(errors='replace'))[:350]);print('  rust-source',repr(z['phases']['after'].get('model',{}).get('source',''))[:350])
for name in ['tim25-results.json','tim25-results.json.gz','tim25-transport.json']:shutil.copy2(C/name,O/name)
dump(S/'fresh25-results.json',results);dump(O/'environment.json',env)
dump(O/'verification.json',dict(resources=26,calls=50,output_gzip_sha256=sha(blob),output_json_sha256=sha(raw),request_sha256=env['request_sha256'],manifest_sha256=env['manifest_sha256'],no_timeouts=True))
