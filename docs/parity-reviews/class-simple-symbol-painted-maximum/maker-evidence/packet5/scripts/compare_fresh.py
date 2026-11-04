# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,os,subprocess,xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
s=Path(__file__).resolve().parent;old=Path('/Users/marcelo/Library/Caches/rustuml-class-cff4');sha=lambda b:hashlib.sha256(b).hexdigest()
bundle=json.loads((s/'reference-results.json').read_bytes());expected={r['name']:r for r in json.loads((s/'fresh-manifest.json').read_text())};assert len(bundle['results'])==len(expected)==20;assert {r['name'] for r in bundle['results']}==set(expected)
transport=json.loads(Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay/class20-transport.json').read_bytes());packed=(s/'reference-results.json.gz').read_bytes();assert base64.b64decode(transport['data'],validate=True)==packed and transport['sha256']==sha(packed);assert gzip.decompress(packed)==(s/'reference-results.json').read_bytes()
assert bundle['environment']['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
env=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
records=[]
for r in bundle['results']:
 name=Path(r['name']);assert name.name==str(name)
 source=Path(expected[str(name)]['source']);data=source.read_bytes();assert sha(data)==r['source_sha256']==expected[str(name)]['source_sha256']
 p=s/'fresh-pairs'/name;p.parent.mkdir(exist_ok=True);p.write_bytes(data)
 for stream,suffix in [('stdout','.svg'),('stderr','.java.stderr')]:
  b=base64.b64decode(r[stream+'_base64'],validate=True);assert sha(b)==r[stream+'_sha256'];p.with_suffix(suffix).write_bytes(b)
 assert r['exit_code']==0 and not r['timed_out'] and r['timezone']=='GMT+11:00' and '-Djava.awt.headless=true' in r['java_argv']
 xml_error=None
 try:assert ET.fromstring(p.with_suffix('.svg').read_bytes()).tag=='{http://www.w3.org/2000/svg}svg'
 except (ET.ParseError,AssertionError) as e:xml_error=str(e)
 row={'name':str(name),'source_sha256':sha(data),'reference_sha256':r['stdout_sha256'],'reference_xml_error':xml_error,'axes':expected[str(name)]['axes']}
 for variant,binary in [('before',old/'corrected-rustuml'),('after',s/'final-rustuml')]:
  runs=[]
  for flag,suffix in [('-tsvg',f'.{variant}.svg'),('--yaml',f'.{variant}.yaml')]:
   cmd=[str(binary),flag,str(source)];c=subprocess.run(cmd,env=env,capture_output=True,timeout=60);p.with_suffix(suffix).write_bytes(c.stdout);p.with_suffix(suffix+'.stderr').write_bytes(c.stderr);runs.append(dict(command=cmd,exit=c.returncode,stderr=c.stderr.decode(errors='replace'),stdout_sha256=sha(c.stdout)))
  cmd=[str(old/'check_svg'),str(p.with_suffix('.svg')),str(p.with_suffix(f'.{variant}.svg'))];c=subprocess.run(cmd,capture_output=True,timeout=30);p.with_suffix(f'.{variant}.diff').write_bytes(c.stdout);p.with_suffix(f'.{variant}.diff.stderr').write_bytes(c.stderr)
  status='invalid_xml' if xml_error else 'invalid_reference' if c.returncode==2 else 'process_error' if runs[0]['exit'] else 'pass' if c.returncode==0 else 'mismatch' if c.returncode==1 else 'comparison_error'
  row[variant]=dict(status=status,runs=runs,comparison_command=cmd,comparison_exit=c.returncode,comparison_stdout=c.stdout.decode(errors='replace'),comparison_stderr=c.stderr.decode(errors='replace'))
 row['model_bytes_unchanged']=p.with_suffix('.before.yaml').read_bytes()==p.with_suffix('.after.yaml').read_bytes()
 records.append(row)
report=dict(revision='96a4bde220eddbc3faf135cad6d13f338b7d7ed5',before_binary_sha256=sha((old/'corrected-rustuml').read_bytes()),after_binary_sha256=sha((s/'final-rustuml').read_bytes()),environment={k:env[k] for k in ['PLANTUML_STDLIB','RUSTUML_DEBUG']},reference_environment=bundle['environment'],counts={v:dict(Counter(r[v]['status'] for r in records)) for v in ['before','after']},results=records)
(s/'fresh-comparison.json').write_text(json.dumps(report,indent=2)+'\n');print(report['counts']);print('unchanged models',sum(r['model_bytes_unchanged'] for r in records));print('failures',[(r['name'],r['after']['status']) for r in records if r['after']['status']!='pass'])
