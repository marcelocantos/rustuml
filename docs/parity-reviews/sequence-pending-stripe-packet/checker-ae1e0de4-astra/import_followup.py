# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,base64,gzip,subprocess,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent;sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha((C/'followup-reference-results.json.gz').read_bytes())=='1890b45ff1d19c7fa25731ced0bad3a2d858b1284c827137689d94ac2e35859f'
assert sha((C/'followup-reference-transport.json').read_bytes())=='1a0a2edec31c4cf7c9d2e0c8541d56c4a1cace340cc0a7283aa4f278fca65e65'
assert gzip.decompress((C/'followup-reference-results.json.gz').read_bytes())==(C/'followup-reference-results.json').read_bytes()
m=json.loads((C/'followup-reference-manifest.json').read_bytes());r=json.loads((C/'followup-reference-results.json').read_bytes());assert len(m['cases'])==len(r['results'])==10
assert r['environment']['java_revision']=='71806a23780b04a5ccde2f8ceb5121edad5eb711';assert r['environment']['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
rows=[];manifest=[]
for a,b in zip(m['cases'],r['results']):
 assert a['name']==b['name'];p=Path(a['source']);body=p.read_bytes();assert sha(body)==a['source_sha256']==b['source_sha256']==b['stdin_sha256'];assert len(body)==b['stdin_bytes']==b['source_bytes'];assert b['requested_argv']==a['invocation'];assert b['argv']==[b['cwd'] if x=='PRIVATE_CWD' else x for x in a['invocation']];assert b['timezone']=='GMT+11:00';assert b['env_overrides']=={};assert not b['timed_out']
 stdout=base64.b64decode(b['stdout_base64']);stderr=base64.b64decode(b['stderr_base64']);assert sha(stdout)==b['stdout_sha256'];assert sha(stderr)==b['stderr_sha256'];svg=p.with_suffix('.svg');svg.write_bytes(stdout);p.with_suffix('.stderr').write_bytes(stderr)
 root=ET.fromstring(stdout);texts=[''.join(x.itertext()) for x in root.iter() if x.tag.endswith('}text')]
 diag='After create command, you have to send a message to' in ' '.join(texts)
 cmp=subprocess.run([str(C/'after-bin/compare_cli'),str(svg),str(svg)],capture_output=True)
 valid=root.get('data-diagram-type')=='SEQUENCE' and cmp.returncode==0 and not diag
 assert valid or diag,(a['name'],root.attrib,texts)
 rows.append(dict(name=a['name'],intended_valid=a['intended_valid'],observed_valid=valid,diagram_type=root.get('data-diagram-type'),error_diagnostic=diag,exit=b['exit_code'],comparator_self_exit=cmp.returncode,reference_sha256=sha(stdout)))
 manifest.append(dict(source=str(p),golden=str(svg),cohort='followup10',mechanism=a['mechanism'],axes=a['axes'],intended_valid=a['intended_valid'],observed_valid=valid))
(C/'followup-reference-classification.json').write_text(json.dumps(rows,indent=2));(C/'followup10.json').write_text(json.dumps(manifest,indent=2));print('valid',sum(x['observed_valid'] for x in rows),'invalid',sum(not x['observed_valid'] for x in rows),'disagreements',sum(x['intended_valid']!=x['observed_valid'] for x in rows))
