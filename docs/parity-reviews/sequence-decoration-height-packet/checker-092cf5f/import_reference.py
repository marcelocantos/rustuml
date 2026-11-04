# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64, collections, gzip, hashlib, json, xml.etree.ElementTree as ET
from pathlib import Path
cache=Path(__file__).parent
manifest=json.loads((cache/'reference-manifest.json').read_text())
raw=(cache/'reference-results.json.gz').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='a8e9b5cda3b6c1d6b5663f0551f10b0425a2d459d34c9738192862445a61dbce'
assert gzip.decompress(raw)==(cache/'reference-results.json').read_bytes()
reference=json.loads(gzip.decompress(raw))
assert reference['environment']['jar_sha256']==manifest['jar_sha256']
assert reference['environment']['headless']
assert reference['environment']['manifest_sha256']==hashlib.sha256((cache/'reference-manifest.json').read_bytes()).hexdigest()
requests={r['name']:r for r in manifest['cases']}
assert len(reference['results'])==len(requests)==31
assert {r['name'] for r in reference['results']}==set(requests)
rows=[]; classes=[]
for r in reference['results']:
    m=requests[r['name']];source=Path(m['source']);data=source.read_bytes()
    assert len(data)==r['source_bytes']==m['bytes']
    assert hashlib.sha256(data).hexdigest()==r['source_sha256']==m['source_sha256']
    assert r['requested_argv']==m['invocation'] and r['requested_cwd']==manifest['remote_private_directory']
    assert r['cwd']=='/Users/marcelo/rustuml-reference/scratch/sequence-review-092cf5f-astra'
    assert r['timezone']=='GMT+11:00' and not r['timed_out']
    assert r['argv']==r['actual_wrapper_argv'] and '-Djava.awt.headless=true' in r['actual_java_argv']
    for stream,extension in [('stdout','.svg'),('stderr','.stderr')]:
        b=base64.b64decode(r[stream+'_base64'],validate=True)
        assert hashlib.sha256(b).hexdigest()==r[stream+'_sha256']
        p=source.with_suffix(extension); assert not p.exists();p.write_bytes(b);p.chmod(0o444)
    output=source.with_suffix('.svg').read_bytes()
    try:
        xml=ET.fromstring(output); xml_valid=True; dtype=xml.get('data-diagram-type')
    except ET.ParseError:
        xml_valid=False; dtype=None
    error=b'Syntax Error' in output or dtype=='ERROR'
    classification=('valid_sequence' if r['exit_code']==0 and xml_valid and dtype=='SEQUENCE' and not error else 'empty_output' if not output else 'java_error' if r['exit_code']!=0 or error else 'non_sequence_output')
    classes.append(dict(name=r['name'],exit=r['exit_code'],xml=xml_valid,diagram_type=dtype,classification=classification,source_bytes=len(data),stdout_bytes=len(output),stderr_bytes=source.with_suffix('.stderr').stat().st_size))
    rows.append(dict(source=str(source),golden=str(source.with_suffix('.svg')),cohort='checker_092_fresh31',name=r['name'],axes=m['axes'],reference_classification=classification))
(cache/'fresh-manifest-private.json').write_text(json.dumps(rows,indent=2)+'\n')
(cache/'reference-classification.json').write_text(json.dumps(classes,indent=2)+'\n')
print(collections.Counter(r['classification'] for r in classes))
print([r for r in classes if r['classification']!='valid_sequence'])
