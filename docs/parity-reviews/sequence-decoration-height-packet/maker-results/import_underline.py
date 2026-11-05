# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,xml.etree.ElementTree as ET
from pathlib import Path
p=Path(__file__).parent
t=json.loads((p/'underline-reference-transport.json').read_text());raw=(p/'underline-reference-results.json.gz').read_bytes();assert hashlib.sha256(raw).hexdigest()==t['sha256'];assert raw==base64.b64decode(t['data']);assert gzip.decompress(raw)==(p/'underline-reference-results.json').read_bytes()
refs=json.loads(gzip.decompress(raw));expected={r['name']:r for r in json.loads((p/'underline-manifest.json').read_text())['cases']};assert {r['name'] for r in refs['results']}==set(expected)
rows=[];classes=[]
for r in refs['results']:
 source=Path(expected[r['name']]['source']);assert hashlib.sha256(source.read_bytes()).hexdigest()==r['source_sha256']==expected[r['name']]['source_sha256']
 for stream in ['stdout','stderr']:
  data=base64.b64decode(r[stream+'_base64']);assert hashlib.sha256(data).hexdigest()==r[stream+'_sha256'];source.with_suffix('.svg' if stream=='stdout' else '.stderr').write_bytes(data)
 svg=source.with_suffix('.svg');xml=True
 try:ET.fromstring(svg.read_bytes())
 except ET.ParseError:xml=False
 error=b'Syntax Error' in svg.read_bytes() or b'data-diagram-type="ERROR"' in svg.read_bytes();valid=xml and not error and r['exit_code']==0
 classes.append(dict(name=r['name'],exit=r['exit_code'],xml=xml,error=error,reference_valid=valid,intended_negative=False))
 rows.append(dict(source=str(source),golden=str(svg),cohort='underline4',reference_valid=valid))
(p/'underline-classification.json').write_text(json.dumps(classes,indent=2)+'\n');(p/'underline4.json').write_text(json.dumps(rows,indent=2)+'\n');print(classes)
