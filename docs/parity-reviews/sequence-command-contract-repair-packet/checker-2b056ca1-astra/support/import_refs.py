# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,collections,gzip,hashlib,json,re,sys,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(sys.argv[1]);expected_gz=sys.argv[2];expected_transport=sys.argv[3];R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest()
z=(C/'reference-results.json.gz').read_bytes();assert sha(z)==expected_gz;raw=gzip.decompress(z);assert raw==(C/'reference-results.json').read_bytes();transport=(C/'reference-transport.json').read_bytes();assert sha(transport)==expected_transport;assert base64.b64decode(json.loads(transport)['data'],validate=True)==z
v=json.loads(raw);mraw=(C/'reference-manifest.json').read_bytes();m=json.loads(mraw);e=v['environment'];assert e['manifest_sha256']==sha(mraw);assert e['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053';assert e['java_revision']=='71806a23780b04a5ccde2f8ceb5121edad5eb711';assert e['headless'] and e['timezone']=='GMT+11:00' and e['env_overrides']=={}
original=json.loads(gzip.decompress((C/'reference-input-bundle.json.gz').read_bytes()));relay=json.loads(gzip.decompress((C/'parent-relay-input.json.gz').read_bytes()));assert all(relay[k]==x for k,x in original.items());assert base64.b64decode(relay['manifest_base64'])==mraw
assert len(v['results'])==len(m['cases']);rows=[]
for req,res in zip(m['cases'],v['results']):
 assert req['name']==res['name'] and re.fullmatch(r'[a-z0-9_]+',req['name']);p=Path(req['source']);body=p.read_bytes();assert sha(body)==req['source_sha256']==res['source_sha256']==res['stdin_sha256'];assert len(body)==res['source_bytes']==res['stdin_bytes'];assert not res['timed_out'];assert res['cwd']==e['worker']+'/new';assert res['timezone']=='GMT+11:00';assert res['actual_wrapper_argv']==[res['cwd'] if x=='PRIVATE_CWD' else x for x in req['invocation']];assert res['actual_java_argv']==[e['java'],'-Djava.awt.headless=true','-Dfile.encoding=UTF-8','-Duser.language=en','-Duser.country=AU','-Duser.timezone=GMT+11:00','-jar',e['jar'],'-pipe','-tsvg'];assert res['env_overrides']=={}
 streams={k:base64.b64decode(res[k+'_base64'],validate=True) for k in ['stdout','stderr']};assert all(sha(x)==res[k+'_sha256'] for k,x in streams.items())
 for k,suffix in [('stdout','.svg'),('stderr','.stderr')]:p.with_suffix(suffix).write_bytes(streams[k])
 try:root=ET.fromstring(streams['stdout']);xml=True;diagram=root.attrib.get('data-diagram-type')
 except ET.ParseError:xml=False;diagram=None
 error_svg=b'An error has occured' in streams['stdout'];export=b'An invalid XML character' in streams['stderr']
 valid=res['exit_code']==0 and xml and diagram=='SEQUENCE' and not error_svg
 if valid:outcome='valid_sequence'
 elif res['exit_code']==200:outcome='java_syntax_error_with_export_failure' if export else 'java_syntax_error'
 elif export:outcome='java_export_error'
 else:outcome='other_nonpositive'
 d=R/'test-diagrams/perturbations'/req['mechanism'];d.mkdir(parents=True,exist_ok=True);source=d/p.name;golden=source.with_suffix('.svg');assert not source.exists() and not golden.exists();source.write_bytes(body);golden.write_bytes(streams['stdout'])
 rows.append(dict(name=req['name'],source=str(source.relative_to(R)),golden=str(golden.relative_to(R)),cohort='checker'+str(len(m['cases'])),mechanism=req['mechanism'],axes=req['axes'],intended_valid=req['intended_valid'],observed_valid=valid,reference_outcome=outcome,syntax_rejection_obligation=res['exit_code']==200,java_exit=res['exit_code'],xml_valid=xml,diagram_type=diagram,export_failure=export,source_sha256=sha(body),golden_sha256=sha(streams['stdout']),stderr_sha256=sha(streams['stderr']),java_argv=res['actual_java_argv'],wrapper_argv=res['actual_wrapper_argv'],reference_cwd=res['cwd']))
(C/'fresh-pairs.json').write_text(json.dumps(rows,indent=2)+'\n');validation=dict(entries=len(rows),manifest_sha256=sha(mraw),gzip_sha256=sha(z),transport_sha256=sha(transport),environment=e,outcomes=dict(collections.Counter(r['reference_outcome'] for r in rows)),intent_disagreements=[r['name'] for r in rows if r['intended_valid']!=r['observed_valid']],source_hashes_verified=True,all_raw_streams_verified=True,argv_json_arrays_verified=True,original_envelope_fields_equal=True,limits='Java argv derived, not process-traced. Historical OpenJDK21.0.5/Graphviz12.2 equivalence unproved. Intent is never rewritten from observation.')
(C/'reference-validation.json').write_text(json.dumps(validation,indent=2)+'\n');print(validation['outcomes']);print('disagreements',len(validation['intent_disagreements']))
