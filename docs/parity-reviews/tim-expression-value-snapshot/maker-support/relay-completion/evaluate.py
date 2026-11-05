# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Verify the remote byte relay and compare its outputs with the local CLI. No JVM."""
import base64,gzip,hashlib,json,os,subprocess
from pathlib import Path
import xml.etree.ElementTree as ET
import yaml
ROOT=Path.cwd();SUP=ROOT/'docs/parity-reviews/tim-expression-value-snapshot/maker-support'
S=Path('/tmp/rustuml-tim4');OUT=S/'relay-completion';OUT.mkdir(exist_ok=True)
CACHE=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay/tim17-results.json')
def sha(b):return hashlib.sha256(b).hexdigest()
def save(p,b):p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
def dump(p,v):save(p,(json.dumps(v,indent=2)+'\n').encode())
blob=CACHE.with_suffix('.json.gz').read_bytes();assert sha(blob)=='cd50f6bf14cd85993a8e584280d4db3c45133ced6a74ca6682c0d3da0704f7ae'
raw=gzip.decompress(blob);assert raw==CACHE.read_bytes();relay=json.loads(raw)
assert relay['environment']['input_compressed_sha256']=='4cd171c5c3b0e08205875b4e845f7434cd03c41a6ac875ea539541218f400539'
manifest=json.loads((S/'new-manifest.json').read_text());expected={r['source']:r for r in manifest}
rows=relay['results'];assert len(rows)==len(expected)==17 and {r['source'] for r in rows}==set(expected)
request=json.loads((S/'remote-request.json').read_text());resources=relay['environment']['resources'];assert resources==request['resources']
for r in resources:
 data=Path(r['local_path']).read_bytes();assert sha(data)==r['sha256'];assert len(data)==r['bytes']
 if r['remote_relative_path'].startswith('new/'):
  assert (SUP/'inputs'/Path(r['remote_relative_path']).relative_to('new')).read_bytes()==data
provenance=json.loads((SUP/'provenance.json').read_text())
binary=S/'final-rustuml';comparator=S/'probe'
assert sha(binary.read_bytes())==provenance['binary_sha256'];assert sha(comparator.read_bytes())==provenance['comparator_sha256']
save(OUT/'tim17-results.json.gz',blob);dump(OUT/'remote-environment.json',relay['environment'])
results=[]
for row in rows:
 p=Path(row['source']);source=p.read_bytes();assert sha(source)==row['source_sha256'];case=OUT/p.stem;case.mkdir(exist_ok=True)
 java=base64.b64decode(row['stdout_base64'],validate=True);jerr=base64.b64decode(row['stderr_base64'],validate=True);jprobe=base64.b64decode(row['probe']['stdout_base64'],validate=True);perr=base64.b64decode(row['probe']['stderr_base64'],validate=True)
 for name,b in [('java.svg',java),('java.stderr',jerr),('java.source',jprobe),('source.stderr',perr)]:save(case/name,b)
 env=dict(os.environ,RUSTUML_DEBUG=expected[row['source']]['debug'],TZ=expected[row['source']]['zone'],JAVA_TOOL_OPTIONS=(os.environ.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip())
 calls=[]
 for mode,args in [('svg',[]),('yaml',['--yaml'])]:
  cmd=[str(binary),*args,'-pipe'];r=subprocess.run(cmd,input=source,cwd=p.parent,env=env,capture_output=True,timeout=90);save(case/('rust.'+mode),r.stdout);save(case/(mode+'.stderr'),r.stderr);calls.append(dict(argv=cmd,cwd=str(p.parent),exit_code=r.returncode))
 cmd=[str(comparator),'compare',str(case/'java.svg'),str(case/'rust.svg')];c=subprocess.run(cmd,capture_output=True,timeout=90);save(case/'comparison.txt',c.stdout+c.stderr)
 try:ET.fromstring(java);xml_valid=True
 except ET.ParseError:xml_valid=False
 source_equal=False;source_error=None
 try:
  node=yaml.compose((case/'rust.yaml').read_bytes())
  for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
  value=node.value.encode();save(case/'rust.source',value);source_equal=value==jprobe
 except (TypeError,ValueError,KeyError,AttributeError,yaml.YAMLError) as e:source_error=str(e)
 valid=row['exit_code']==0 and xml_valid
 result=dict(source=row['source'],mechanism=expected[row['source']]['mechanism'],intended_negative=expected[row['source']]['negative'],source_sha256=row['source_sha256'],java_exit=row['exit_code'],java_probe_exit=row['probe']['exit_code'],java_svg_xml_valid=xml_valid,java_valid_reference=valid,cli_exit=calls[0]['exit_code'],comparison_exit=c.returncode,svg_match=valid and calls[0]['exit_code']==0 and c.returncode==0,source_equal=valid and source_equal,source_diagnostic_error=source_error,java_argv=row['argv'],java_probe_argv=row['probe']['argv'],rust_calls=calls,comparison_argv=cmd)
 dump(case/'result.json',result);results.append(result)
 print(p.stem,'java',row['exit_code'],'rust',calls[0]['exit_code'],'svg',result['svg_match'],'source',result['source_equal'],flush=True)
dump(OUT/'results.json',results)
summary=dict(candidate=provenance['candidate'],cases=len(rows),valid_references=sum(r['java_valid_reference'] for r in results),svg_matches=sum(r['svg_match'] for r in results),source_matches=sum(r['source_equal'] for r in results),negative_references=[r['source'] for r in results if not r['java_valid_reference']],valid_cli_errors=[r['source'] for r in results if r['java_valid_reference'] and r['cli_exit']!=0],valid_svg_failures=[r['source'] for r in results if r['java_valid_reference'] and not r['svg_match']],transport_input_compressed_sha256=relay['environment']['input_compressed_sha256'],transport_output_compressed_sha256=sha(blob),resources_verified=len(resources),binary_sha256=provenance['binary_sha256'],comparator_sha256=provenance['comparator_sha256'],local_java_launches=0)
dump(OUT/'summary.json',summary);print(json.dumps(summary,indent=2))
