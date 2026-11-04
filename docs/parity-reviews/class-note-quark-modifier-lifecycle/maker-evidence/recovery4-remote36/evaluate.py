# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,subprocess,xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
s=Path(__file__).resolve().parent
relay=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay')
raw=(relay/'class36-results.json').read_bytes();compressed=(relay/'class36-results.json.gz').read_bytes()
sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha(compressed)=='53ec8058bcc4db68c5746d53834670df0b063c9cebfd611099011044386b1732'
assert gzip.decompress(compressed)==raw
transport=json.loads((relay/'class36-transport.json').read_bytes());assert base64.b64decode(transport['data'],validate=True)==compressed and transport['sha256']==sha(compressed)
bundle=json.loads(raw);manifest=json.loads((relay/'class36-manifest.json').read_bytes());snapshot=json.loads((s/'pending-snapshot.json').read_bytes());before={r['name']:r for r in snapshot['entries']};expected={r['name']:r for r in manifest}
assert len(expected)==len(bundle['results'])==36 and {r['name'] for r in bundle['results']}==set(expected)==set(before)
assert bundle['environment']['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
comparator=s.parent/'check_svg';records=[]
for r in bundle['results']:
 name=Path(r['name']);assert not name.is_absolute() and '..' not in name.parts
 source=Path(expected[str(name)]['source']);data=source.read_bytes();rust=source.with_suffix('.rust.svg').read_bytes()
 assert sha(data)==r['source_sha256']==expected[str(name)]['source_sha256']==before[str(name)]['source_sha256']
 assert sha(rust)==before[str(name)]['rust_svg_sha256']
 stdout=base64.b64decode(r['stdout_base64'],validate=True);stderr=base64.b64decode(r['stderr_base64'],validate=True)
 assert sha(stdout)==r['stdout_sha256'] and sha(stderr)==r['stderr_sha256']
 assert r['exit_code']==0 and not r['timed_out'] and r['timezone']=='GMT+11:00'
 assert r['cwd']==bundle['environment']['worker'] and '-Djava.awt.headless=true' in r['java_argv']
 p=s/'pairs'/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data);p.with_suffix('.svg').write_bytes(stdout);p.with_suffix('.java.stderr').write_bytes(stderr);p.with_suffix('.rust.svg').write_bytes(rust)
 xml_error=None
 try:
  root=ET.fromstring(stdout)
  if root.tag!='{http://www.w3.org/2000/svg}svg':xml_error='Non-SVG XML root: '+root.tag
 except ET.ParseError as e:xml_error=str(e)
 cmd=[str(comparator),str(p.with_suffix('.svg')),str(p.with_suffix('.rust.svg'))];c=subprocess.run(cmd,capture_output=True,timeout=30)
 p.with_suffix('.diff.txt').write_bytes(c.stdout);p.with_suffix('.diff.stderr').write_bytes(c.stderr)
 status='invalid_xml' if xml_error else 'invalid_reference' if c.returncode==2 else 'match' if c.returncode==0 else 'mismatch' if c.returncode==1 else 'comparison_error'
 records.append(dict(name=str(name),source_sha256=sha(data),reference_sha256=sha(stdout),reference_stderr_sha256=sha(stderr),rust_sha256=sha(rust),reference_exit=r['exit_code'],reference_xml_error=xml_error,status=status,comparison_command=cmd,comparison_exit=c.returncode,comparison_stdout=c.stdout.decode(errors='replace'),comparison_stderr=c.stderr.decode(errors='replace')))
result=dict(kind='maker comparison; no acceptance attestation',snapshot_commit=snapshot['snapshot_commit'],production_revision='7bd5c9f5d2f4dee23472ddc8e3ac4b96a3e196e3',new_rust_renders=0,local_java_launches=0,new_suite_runs=0,comparator_sha256=sha(comparator.read_bytes()),reference_bundle_sha256=sha(raw),reference_compressed_sha256=sha(compressed),environment=bundle['environment'],counts=dict(Counter(r['status'] for r in records)),results=records)
(s/'comparison.json').write_text(json.dumps(result,indent=2)+'\n');print(result['counts'])
for r in records:print(r['status'],r['name'],r['comparison_stdout'].splitlines()[:2])
