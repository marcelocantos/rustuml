# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,hashlib,base64,importlib.util,subprocess,os,xml.etree.ElementTree as ET
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker');R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-9a8dea60')
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
spec=importlib.util.spec_from_file_location('replay',S/'replay.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);m.S=S;m.R=R
manifest=json.loads((S/'manifest.json').read_bytes());request=json.loads((S/'request.json').read_bytes());transport=(S/'reference-results.json.gz').read_bytes();raw=json.loads(gzip.decompress(transport));env=raw['environment'];worker=env['worker'];jar='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
assert env['jar_sha256']==jar;assert env['request_sha256']==sha((S/'request.json').read_bytes());assert env['manifest_sha256']==sha((S/'manifest.json').read_bytes());assert env['headless'] is True
assert len(raw['resources'])==len(request['resources'])==len(manifest)==38
for expected,actual in zip(request['resources'],raw['resources']):
 for key in ['local_path','remote_relative_path','sha256','bytes']:assert expected[key]==actual[key],(key,expected,actual)
 assert sha(Path(expected['local_path']).read_bytes())==expected['sha256']
assert len(raw['results'])==len(manifest)
observations=[]
for membership,contract,r in zip(manifest,request['case_contracts'],raw['results']):
 p=Path(membership['source']);name=p.stem;data=p.read_bytes();d=S/'fresh-reference'/name;d.mkdir(parents=True,exist_ok=True)
 assert r['source']==str(p) and r['source_sha256']==sha(data) and r['stdin_sha256']==sha(data) and r['stdin_bytes']==len(data)
 assert r['timezone']=='GMT+11:00' and r['cwd']==worker+'/new' and r['requested_cwd']=='<worker>/new'
 expected=[env['wrapper'],'--cwd',worker+'/new','--timezone','GMT+11:00','--','-pipe','-tsvg'];assert r['argv']==r['actual_wrapper_argv']==expected
 java=[env['java'],'-Djava.awt.headless=true','-Dfile.encoding=UTF-8','-Duser.language=en','-Duser.country=AU','-Duser.timezone=GMT+11:00','-jar',env['jar'],'-pipe','-tsvg'];assert r['java_argv']==r['actual_java_argv']==java
 assert r['requested_env_overrides']==r['env_overrides']=={}
 for stream in ['stdout','stderr']:
  b=base64.b64decode(r[stream+'_base64'],validate=True);assert sha(b)==r[stream+'_sha256'];(d/('java.svg.'+stream)).write_bytes(b)
 dump(d/'reference-command.json',{k:v for k,v in r.items() if not k.endswith('_base64')})
 gold=d/'java.svg.stdout';b=gold.read_bytes();err=(d/'java.svg.stderr').read_bytes()
 ref=m.call([S/'probe','compare',gold,gold],R,os.environ,None,d,'reference-validation')
 try:
  root=ET.fromstring(b);xml=True;text=' '.join(root.itertext());welcome='Welcome to PlantUML!' in text;factory=root.get('data-diagram-type')
 except ET.ParseError:xml=False;welcome=False;factory=None;text=''
 category=('timeout' if r['timed_out'] else 'empty_output' if not b else 'syntax_negative' if ref['exit']==2 or factory=='ERROR' else 'runtime_failure' if b'Exception' in err or r['exit_code']!=0 else 'welcome' if welcome else 'invalid_xml' if not xml else 'positive_reference')
 valid=category=='positive_reference'
 row=dict(id='checker38/'+name,suite='checker38',membership=dict(membership,reference_valid=valid),cwd=str(S/'new'),source_capture=str(p),golden_capture=str(gold))
 phases={}
 for phase in ['before','after']:
  v=m.measure(row,phase);v['helper_reference_valid']=v['reference_valid'];v['reference_valid']=valid;v['svg_match']=valid and v['svg']['exit']==0 and v['compare']['exit']==0;phases[phase]=v
 standing=m.call([S/'standing-slice',p,gold],R,dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib'),None,d,'standing')
 observation=dict(name=name,row=membership,classification=category,reference_valid=valid,reference_exit=r['exit_code'],reference_timed_out=r['timed_out'],reference_xml_valid=xml,reference_factory=factory,reference_comparator_exit=ref['exit'],phases=phases,standing=standing,source_sha256=sha(data),golden_sha256=sha(b))
 observations.append(observation);dump(d/'result.json',observation);print(name,category,phases['before']['svg_match'],phases['after']['svg_match'],standing['exit'],flush=True)
dump(S/'fresh-results.json',observations);dump(S/'reference-validation.json',dict(validated=True,cases=len(observations),resources=len(raw['resources']),returned_gzip_sha256=sha(transport),request_sha256=sha((S/'request.json').read_bytes()),bundle_sha256=sha((S/'input-bundle.json.gz').read_bytes()),checks=['source and resource hashes and lengths','all stream hashes and statuses','exact wrapper and Java argv','private cwd timezone env headless JAR','membership order and independently classified validity']))
