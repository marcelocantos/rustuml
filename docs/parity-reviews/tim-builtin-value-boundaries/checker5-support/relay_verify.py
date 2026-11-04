# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,shutil,sys,xml.etree.ElementTree as ET
from pathlib import Path
S=Path(__file__).parent; R=Path.cwd(); sha=lambda b:hashlib.sha256(b).hexdigest()
supplement=len(sys.argv)>1
request_name='supplement-request.json' if supplement else 'remote-request.json'
manifest_name='supplement-manifest.json' if supplement else 'new-manifest.json'
batch='tim5' if supplement else 'tim24'
outdir='relay5' if supplement else 'relay24'
expected_hash='08e901e88fe5e116f94583e12e9c25e0665e681a9ae3bd0d94bd67f611f7242f' if supplement else 'ad5c57f022f0740737b32bc117a205941f15419e3af561b9fd2a03b9e9325f82'
req=json.loads((S/request_name).read_bytes());manifest=json.loads((S/manifest_name).read_bytes())
p=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay')/(batch+'-results.json.gz');assert sha(p.read_bytes())==expected_hash;shutil.copy2(p,S/p.name)
v=json.loads(gzip.decompress(p.read_bytes()));e=v['environment'];assert e['jar_sha256']==req['jar_sha256'];assert e['request_sha256']==sha((S/request_name).read_bytes());assert e['manifest_sha256']==sha((S/manifest_name).read_bytes());assert e['headless'] is True
assert v['resources']==req['resources'];assert len(v['results'])==len(manifest)==req['case_count']
for a in v['resources']:
 b=Path(a['local_path']).read_bytes();assert sha(b)==a['sha256'] and len(b)==a['bytes']
contracts={Path(c['source']).stem:c for c in req['case_contracts']};results=[]
for row in v['results']:
 c=contracts[row['id']];assert row['source_sha256']==c['source_sha256']; assert row['source']==c['source']
 d=S/outdir/row['id'];d.mkdir(parents=True,exist_ok=True)
 for mode,call in [('svg',row),('source',row['probe'])]:
  argv=[a.replace('<worker>',e['worker']) for a in c['svg_argv' if mode=='svg' else 'probe_argv']];argv[0]=req['wrapper'];assert call['argv']==argv
  assert call['cwd']==c['cwd'].replace('<worker>',e['worker']);assert call['timezone']==c['timezone'];assert call['env_overrides']==c['env_overrides'];assert not call['timed_out']
  for prop in ['-Djava.awt.headless=true','-Dfile.encoding=UTF-8','-Duser.language=en','-Duser.country=AU','-Duser.timezone='+c['timezone']]:assert prop in call['java_argv']
  for stream in ['stdout','stderr']:
   b=base64.b64decode(call[stream+'_base64'],validate=True);assert sha(b)==call[stream+'_sha256'];(d/f'java.{mode}.{stream}').write_bytes(b)
 (d/'commands.json').write_text(json.dumps({k:val for k,val in row.items() if not k.endswith('_base64') and k!='probe'}|{'probe':{k:val for k,val in row['probe'].items() if not k.endswith('_base64')}},indent=2)+'\n')
 b=(d/'java.svg.stdout').read_bytes()
 try:root=ET.fromstring(b);valid_xml=True;error_svg=root.attrib.get('data-diagram-type')=='ERROR' or 'syntax error' in b.decode().lower() or b'An error has occured' in b or b'Welcome to PlantUML' in b
 except ET.ParseError:valid_xml=False;error_svg=False
 results.append(dict(id=row['id'],java_exit=row['exit_code'],probe_exit=row['probe']['exit_code'],xml_valid=valid_xml,error_svg=error_svg,reference_valid=row['exit_code']==0 and valid_xml and not error_svg))
 print(row['id'],results[-1],repr((d/'java.source.stdout').read_bytes().decode()[:450]))
(S/(batch+'-verification.json')).write_text(json.dumps(dict(resources=len(v['resources']),calls=2*len(manifest),environment=e,results=results),indent=2)+'\n')
