# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,base64,hashlib,subprocess,xml.etree.ElementTree as ET
from pathlib import Path
S=Path(__file__).parent;sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
gzip_bytes=(S/'reference-results.json.gz').read_bytes();assert sha(gzip_bytes)=='275dcf6c83a99246c947016edafc7f92e4d8b9c4c154961f69e7f106dea78c45'
raw=gzip.decompress(gzip_bytes);assert raw==(S/'reference-results.json').read_bytes();v=json.loads(raw);env=v['environment'];req=json.loads((S/'request.json').read_bytes());manifest=json.loads((S/'manifest.json').read_bytes());original=json.loads(gzip.decompress((S/'input-bundle.json.gz').read_bytes()))
assert env['jar_sha256']==req['jar_sha256'];assert env['java_revision']==req['java_revision'];assert env['headless'] is True
assert env['request_sha256']==sha((S/'request.json').read_bytes());assert env['manifest_sha256']==sha((S/'manifest.json').read_bytes());assert env['original_input_bundle_sha256']==sha((S/'input-bundle.json.gz').read_bytes());assert env['input_compressed_sha256']==sha((S/'parent-relay-input.json.gz').read_bytes())
assert original['request']==req and original['manifest']==manifest
assert v['resources']==req['resources'];assert len(v['resources'])==len(v['results'])==len(manifest)==26
for file,r in zip(original['files'],req['resources']):
 b=base64.b64decode(file['base64'],validate=True);assert b==Path(r['local_path']).read_bytes();assert len(b)==r['bytes'];assert sha(b)==r['sha256'];assert file['local_path']==r['local_path'];assert file['remote_relative_path']==r['remote_relative_path']
rows=[];observations=[]
for c,contract,result in zip(manifest,req['case_contracts'],v['results']):
 p=Path(c['source']);b=p.read_bytes();d=S/'fresh-reference'/p.stem;d.mkdir(parents=True,exist_ok=True)
 assert result['source']==str(p) and result['source_sha256']==result['stdin_sha256']==sha(b);assert result['stdin_bytes']==len(b)
 assert result['cwd']==env['worker']+'/new' and result['requested_cwd']=='<worker>/new';assert result['timezone']=='GMT+11:00';assert result['env_overrides']==result['requested_env_overrides']=={}
 argv=[env['wrapper'],'--cwd',env['worker']+'/new','--timezone','GMT+11:00','--','-pipe','-tsvg'];assert result['argv']==result['actual_wrapper_argv']==argv
 java=[env['java'],'-Djava.awt.headless=true','-Dfile.encoding=UTF-8','-Duser.language=en','-Duser.country=AU','-Duser.timezone=GMT+11:00','-jar',env['jar'],'-pipe','-tsvg'];assert result['java_argv']==result['actual_java_argv']==java
 for stream in ['stdout','stderr']:
  data=base64.b64decode(result[stream+'_base64'],validate=True);assert sha(data)==result[stream+'_sha256'];(d/('java.svg.'+stream)).write_bytes(data)
 dump(d/'reference-command.json',{k:x for k,x in result.items() if not k.endswith('_base64')})
 gold=d/'java.svg.stdout';q=subprocess.run([str(S/'probe'),'compare',str(gold),str(gold)],capture_output=True);(d/'reference-validation.stdout').write_bytes(q.stdout);(d/'reference-validation.stderr').write_bytes(q.stderr)
 root=ET.fromstring(gold.read_bytes());text=' '.join(root.itertext());stderr=(d/'java.svg.stderr').read_bytes();factory=root.get('data-diagram-type')
 category=('timeout' if result['timed_out'] else 'runtime_error_svg' if 'NumberFormatException' in text or b'Exception' in stderr else 'welcome' if 'Welcome to PlantUML!' in text else 'error_svg' if q.returncode==2 or factory=='ERROR' else 'process_error' if result['exit_code']!=0 else 'valid_diagram')
 valid=category=='valid_diagram';rows.append(dict(id='fresh26/'+p.stem,suite='fresh26',membership=dict(c,reference_valid=valid),cwd=str(p.parent),source_capture=str(p),source_sha256=sha(b),golden_capture=str(gold),golden_sha256=sha(gold.read_bytes())))
 o=dict(name=p.stem,intended=c['intended_validity'],classification=category,valid=valid,factory=factory,reference_exit=result['exit_code'],reference_comparator_exit=q.returncode,reference_timed_out=result['timed_out'],source_sha256=sha(b),golden_sha256=sha(gold.read_bytes()),visible_text=text);observations.append(o);print(p.stem,category,factory,result['exit_code'],flush=True)
dump(S/'fresh26-membership.json',rows);dump(S/'fresh-classification.json',observations);dump(S/'reference-validation.json',dict(all_checks_passed=True,resources=26,calls=26,valid=sum(x['valid'] for x in observations),request_sha256=sha((S/'request.json').read_bytes()),return_sha256=sha(gzip_bytes),java_argv_provenance='Derived from pinned wrapper construction, not process-traced',environment=env))
