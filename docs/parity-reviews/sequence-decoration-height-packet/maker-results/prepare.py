# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,base64,xml.etree.ElementTree as ET
from pathlib import Path
p=Path(__file__).parent
rows=json.loads(Path('docs/parity-reviews/sequence-corrective-b9/maker-results/all-inputs.json').read_text())
fresh=json.loads(Path('docs/parity-reviews/sequence-corrective-b9/checker-ef020/pairs.json').read_text())
for r in fresh:
 if r['fresh']: rows.append(dict(source=r['source'],golden=r['golden'],cohort='checker_ef020_fresh37'))
new=[];classes=[]
for r in json.loads((p/'reference-results.json').read_text())['results']:
 source=Path(r['source']);assert hashlib.sha256(source.read_bytes()).hexdigest()==r['source_sha256']
 for stream in ['stdout','stderr']:
  data=base64.b64decode(r[stream+'_base64']);assert hashlib.sha256(data).hexdigest()==r[stream+'_sha256']
  source.with_suffix('.svg' if stream=='stdout' else '.stderr').write_bytes(data)
 svg=source.with_suffix('.svg');xml=True
 try: ET.fromstring(svg.read_bytes())
 except ET.ParseError:xml=False
 error=b'Syntax Error' in svg.read_bytes() or b'data-diagram-type="ERROR"' in svg.read_bytes()
 valid=r['exit_code']==0 and xml and not error
 classes.append(dict(name=r['name'],intended_negative=r['name'].startswith('invalid_'),exit=r['exit_code'],xml=xml,error=error,reference_valid=valid))
 new.append(dict(source=str(source),golden=str(svg),cohort='maker24',reference_valid=valid))
(p/'reference-classification.json').write_text(json.dumps(classes,indent=2)+'\n')
(p/'retained1938.json').write_text(json.dumps(rows,indent=2)+'\n')
(p/'focused61.json').write_text(json.dumps(rows[-37:]+new,indent=2)+'\n')
(p/'all1962.json').write_text(json.dumps(rows+new,indent=2)+'\n')
print(classes)
