# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Validate all immutable remote bytes and independently measure fresh controls."""
import base64
import gzip
import hashlib
import json
import os
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path
from replay import S, R, dump, sha, measure, call

req_bytes = (S/'remote-request.json').read_bytes()
manifest_bytes = (S/'new-manifest.json').read_bytes()
assert sha(req_bytes) == '77fbc289bc49685ab5aa853b62efad4ab5c0ae7d165cec01d4a558adadc3afb6'
assert sha(manifest_bytes) == 'f3f75e45190bf83f926811d00be35dad86a2cdf189bc37c63ea7dae784311fac'
compressed = (S/'reference-results.json.gz').read_bytes()
assert sha(compressed) == '57dd7f3aae20e502a9ac87ed2aabca8042bc43f2bb121a128ddce38a18328113'
raw = gzip.decompress(compressed)
assert raw == (S/'reference-results.json').read_bytes()
transport = json.loads((S/'reference-transport.json').read_bytes())
assert transport['sha256'] == sha(compressed)
assert base64.b64decode(transport['data'],validate=True) == compressed
req = json.loads(req_bytes); manifest = json.loads(manifest_bytes); bundle = json.loads(raw)
env = bundle['environment']; worker = env['worker']
assert env['request_sha256'] == sha(req_bytes) and env['manifest_sha256'] == sha(manifest_bytes)
assert env['jar_sha256'] == req['jar_sha256'] == 'aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
assert env['wrapper'] == req['wrapper'] and env['headless'] is True
assert env['locale']=='en_AU' and env['encoding']=='UTF-8' and env['env_overrides']=={}
input_bundle = (S/'input-bundle.json').read_bytes()
assert sha(input_bundle) == env['original_input_bundle_sha256'] == 'a8b4040b4bedbdff85791315d5e92bcbb4eb6585a60c898060a3a8c55c214e4f'
input_bundle = json.loads(input_bundle)
assert input_bundle['manifest'] == manifest and input_bundle['request'] == req
assert bundle['resources'] == req['resources'] and len(req['resources']) == 24
for resource, packed in zip(req['resources'], input_bundle['files']):
    assert {k:v for k,v in packed.items() if k!='base64'} == resource
    data = Path(resource['local_path']).read_bytes()
    assert sha(data) == resource['sha256'] and len(data) == resource['bytes']
    assert data == base64.b64decode(packed['base64'],validate=True)
assert env['probe_sha256'] == 'fa9dde4b9f18e337741c73fdc23fa8d94034d2341725566e378eab0ac95ad4b4'
expected = {r['source']:r for r in manifest}; contracts = {r['source']:r for r in req['case_contracts']}
assert len(bundle['results'])==len(expected)==20 and {r['source'] for r in bundle['results']}==set(expected)
results=[]
for index, ref in enumerate(bundle['results']):
    row=expected[ref['source']]; p=Path(row['source']); d=S/'fresh-reference'/p.stem; d.mkdir(parents=True,exist_ok=True)
    assert sha(p.read_bytes())==ref['source_sha256'] and len(p.read_bytes())==ref['source_bytes']
    for mode, command in [('svg',ref),('source',ref['probe'])]:
        planned=contracts[ref['source']]['svg_argv' if mode=='svg' else 'probe_argv']
        actual=[a.replace('<worker>',worker) for a in planned]; actual[0]=req['wrapper']
        assert command['argv']==command['actual_wrapper_argv']==actual
        assert command['requested_argv']==planned
        assert command['cwd']==worker+'/new' and command['requested_cwd']=='<worker>/new'
        assert command['env_overrides']==command['requested_env_overrides']==row['env']=={}
        assert command['timezone']==row['zone']=='GMT+11:00' and not command['timed_out']
        java=[env['java'],'-Djava.awt.headless=true','-Dfile.encoding=UTF-8','-Duser.language=en','-Duser.country=AU','-Duser.timezone=GMT+11:00']
        java += ['-jar',env['jar'],'-pipe','-tsvg'] if mode=='svg' else ['--class-path',env['jar']+':'+worker,worker+'/SourceProbe.java',worker+'/new/'+p.name,worker+'/new']
        assert command['java_argv']==command['actual_java_argv']==java
        assert command['source_sha256']==sha(p.read_bytes()) and command['remote_source']==worker+'/new/'+p.name
        stdin=p.read_bytes() if mode=='svg' else b''
        assert command['stdin_sha256']==sha(stdin) and command['stdin_bytes']==len(stdin)
        for stream in ['stdout','stderr']:
            data=base64.b64decode(command[stream+'_base64'],validate=True)
            assert sha(data)==command[stream+'_sha256']
            (d/('java.'+mode+'.'+stream)).write_bytes(data)
    metadata={k:v for k,v in ref.items() if 'base64' not in k and k!='probe'}
    metadata['probe']={k:v for k,v in ref['probe'].items() if 'base64' not in k}
    dump(d/'commands.json',metadata)
    gold=d/'java.svg.stdout'
    validation=subprocess.run([str(S/'probe'),'compare',str(gold),str(gold)],capture_output=True)
    (d/'reference-validation.stdout').write_bytes(validation.stdout); (d/'reference-validation.stderr').write_bytes(validation.stderr)
    try:
        root=ET.fromstring(gold.read_bytes()); xml=True; factory=root.get('data-diagram-type')
        welcome=any('Welcome to PlantUML!' in (n.text or '') for n in root.iter())
    except ET.ParseError:
        xml,welcome,factory=False,False,None
    valid=ref['exit_code']==0 and xml and validation.returncode==0 and not welcome
    category='welcome_truncation' if welcome else 'positive_reference' if valid else 'syntax_negative' if ref['exit_code'] else 'other_nonpositive'
    m=dict(row,reference_valid=valid)
    local=dict(id='fresh20/'+p.stem,suite='fresh20',index=index,membership=m,cwd=str(p.parent),source_capture=str(p),
               golden_capture=str(gold),reference_source_capture=str(d/'java.source.stdout'))
    phases={phase:measure(local,phase) for phase in ['before','after']}
    localenv=dict(os.environ,**phases['after']['env_overrides'])
    diagnostic=call([S/'source-probe',p,p.parent],p.parent,localenv,None,d,'direct-source')
    source_equal=diagnostic['exit']==0 and (d/'direct-source.stdout').read_bytes()==(d/'java.source.stdout').read_bytes()
    result=dict(row=row,classification=category,reference_valid=valid,java_svg_exit=ref['exit_code'],java_probe_exit=ref['probe']['exit_code'],
                xml_valid=xml,factory=factory,reference_comparator_exit=validation.returncode,source_sha256=sha(p.read_bytes()),
                golden_sha256=sha(gold.read_bytes()),java_source_sha256=sha((d/'java.source.stdout').read_bytes()),
                phases=phases,direct_source=diagnostic,direct_source_equal=source_equal)
    results.append(result);dump(d/'result.json',result)
    print(p.stem,category,'before',phases['before']['svg_match'],'after',phases['after']['svg_match'],'CLI source',phases['after'].get('exact_source'),'direct source',source_equal,flush=True)
dump(S/'fresh-results.json',results)
dump(S/'remote-validation.json',dict(resources_verified=24,membership_verified=20,calls_verified=40,request_sha256=sha(req_bytes),manifest_sha256=sha(manifest_bytes),
                                  returned_gzip_sha256=sha(compressed),returned_json_sha256=sha(raw),environment=env,
                                  scope='Captured resource bytes and declared/recorded process contract only; not global runtime or environment equivalence.'))
