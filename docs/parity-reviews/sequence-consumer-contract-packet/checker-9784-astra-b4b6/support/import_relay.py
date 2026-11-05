# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Validate a parent relay response without invoking Java or changing source bytes."""
import base64,gzip,hashlib,json,subprocess,sys,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(sys.argv[1]);result=Path(sys.argv[2]);expected_sha=sys.argv[3];sha=lambda b:hashlib.sha256(b).hexdigest();raw=result.read_bytes();assert sha(raw)==expected_sha
v=json.loads(gzip.decompress(raw));mraw=(C/'reference-manifest.json').read_bytes();m=json.loads(mraw);env=v['environment'];assert env['manifest_sha256']==sha(mraw);assert env['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053';assert env['headless'];assert len(v['results'])==len(m['cases'])
records=[]
for req,r in zip(m['cases'],v['results']):
 p=Path(req['source']);body=p.read_bytes();assert req['name']==r['name'];assert sha(body)==req['source_sha256']==r['source_sha256']==r['stdin_sha256'];assert len(body)==r['source_bytes']==r['stdin_bytes'];assert not r['timed_out'];assert isinstance(r['actual_java_argv'],list);assert r['actual_wrapper_argv']==[r['cwd'] if x=='PRIVATE_CWD' else x for x in req['invocation']];assert r['timezone']=='GMT+11:00'
 assert r['actual_java_argv']==[env['java'],'-Djava.awt.headless=true','-Dfile.encoding=UTF-8','-Duser.language=en','-Duser.country=AU','-Duser.timezone=GMT+11:00','-jar',env['jar'],'-pipe','-tsvg']
 for field,suffix in [('stdout','.svg'),('stderr','.stderr')]:
  data=base64.b64decode(r[field+'_base64'],validate=True);assert sha(data)==r[field+'_sha256'];p.with_suffix(suffix).write_bytes(data)
 text=p.with_suffix('.svg').read_text();stderr=p.with_suffix('.stderr').read_text();xml_error=None;dtype=None
 try:dtype=ET.fromstring(text).attrib.get('data-diagram-type')
 except ET.ParseError as e:xml_error=str(e)
 comparison=subprocess.run([str(C.parents[0]/'independent/after-bin/compare_cli') if C.name=='followup' else str(C/'independent/after-bin/compare_cli'),str(p.with_suffix('.svg')),str(p.with_suffix('.svg'))],capture_output=True,text=True)
 marker=comparison.returncode==2
 valid=r['exit_code']==0 and dtype=='SEQUENCE' and not xml_error and not marker and not stderr
 records.append(dict(req,golden=str(p.with_suffix('.svg')),cohort='checker534',observed_valid=valid,java_exit=r['exit_code'],diagram_type=dtype,xml_error=xml_error,comparator_self_exit=comparison.returncode,java_error_marker=marker,stderr=stderr,source_bytes=len(body),stdout_sha256=r['stdout_sha256'],stderr_sha256=r['stderr_sha256'],actual_wrapper_argv=r['actual_wrapper_argv'],actual_java_argv=r['actual_java_argv'],actual_cwd=r['cwd'],argv_provenance='derived, not process-traced'))
(C/'reference-results.json.gz').write_bytes(raw);(C/'fresh-classification.json').write_text(json.dumps(records,indent=2)+'\n');(C/'relay-validation.json').write_text(json.dumps(dict(response_sha256=sha(raw),manifest_sha256=sha(mraw),cases=len(records),environment=env,observed_valid=sum(r['observed_valid'] for r in records),ambiguous=[r['name'] for r in records if not r['observed_valid'] and not r['java_error_marker']],intention_disagreements=[r['name'] for r in records if r['intended_valid']!=r['observed_valid']]),indent=2)+'\n');print((C/'relay-validation.json').read_text())
