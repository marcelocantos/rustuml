# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,subprocess,sys,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();rows=[];observations=[]
sha=lambda b:hashlib.sha256(b).hexdigest()
for folder in ([C] if sys.argv[1]=='main' else [C/'supplement']):
    manifest=json.loads((folder/'reference-manifest.json').read_text());p=folder/'reference-results.json'
    result=json.loads(p.read_text()) if p.exists() else json.loads(gzip.decompress((folder/'reference-results.json.gz').read_bytes()))
    responses={r['name']:r for r in result['results']};assert len(responses)==len(manifest['cases'])
    for case in manifest['cases']:
        response=responses[case['name']];source=Path(case['source']).read_bytes();assert sha(source)==case['source_sha256']==response['source_sha256']
        stdout=base64.b64decode(response['stdout_base64']);stderr=base64.b64decode(response.get('stderr_base64',''));code=response.get('exit_code',response.get('exit',response.get('returncode')))
        if code is None:raise ValueError(response.keys())
        directory=R/'test-diagrams/perturbations'/case['mechanism'];directory.mkdir(exist_ok=True,parents=True)
        dest=directory/(case['name']+'.puml');golden=dest.with_suffix('.svg');assert not dest.exists() and not golden.exists();dest.write_bytes(source);golden.write_bytes(stdout)
        try:
            doc=ET.fromstring(stdout);xml=True;family=doc.attrib.get('data-diagram-type');xml_error=None
        except ET.ParseError as e:xml=False;family=None;xml_error=str(e)
        # Comparator classification is used unchanged; Java exit is retained
        # independently to distinguish error SVGs from exporter failures.
        q=subprocess.run([str(C/'after-bin/compare_cli'),str(golden),str(golden)],capture_output=True)
        valid=code==0 and xml and q.returncode==0
        row=dict(case,source=str(dest.relative_to(R)),golden=str(golden.relative_to(R)),golden_sha256=sha(stdout),cohort='checker509' if folder==C else 'supplement62',java_exit=code,java_stderr=stderr.decode(errors='replace'),xml_valid=xml,xml_error=xml_error,family=family,observed_valid=valid,reference_self_comparison=q.returncode)
        rows.append(row);observations.append(dict(name=case['name'],response=response,classification=row))
(C/('fresh-'+sys.argv[1]+'-pairs.json')).write_text(json.dumps(rows,indent=2)+'\n');(C/('fresh-'+sys.argv[1]+'-reference-observations.json')).write_text(json.dumps(observations,indent=2)+'\n')
print('imported',len(rows),'observed valid',sum(x['observed_valid'] for x in rows),'intention disagreements',sum(x['observed_valid']!=x['intended_valid'] for x in rows))
