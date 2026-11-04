# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,base64,hashlib,gzip
from pathlib import Path
S=Path(__file__).resolve().parent;m=json.loads((S/'fresh-manifest.json').read_text());out=[]
for old in m['cases']:
 if old['name'].startswith(('actor_','boundary_','component_','rectangle_','class_generic')):
  c=dict(old);c['name']='classmode_'+old['name'];p=S/'fresh'/(c['name']+'.puml');b=Path(old['source']).read_bytes().replace(b'@startuml\n',b'@startuml\nallowmixing\n',1);p.write_bytes(b);p.chmod(0o444);c.update(source=str(p),remote_name=p.name,source_sha256=hashlib.sha256(b).hexdigest(),bytes=len(b),source_base64=base64.b64encode(b).decode());c['axes']=old['axes']+['explicit class factory selection'];out.append(c)
m['cases']=out;m['review']='class-packet6-classmode-60091e6d';m['generation_note']='Original 29 graphical controls lacked allowmixing: mixed class/description negatives or alternate factory, not coverage of class description path. Preserve originals and separately add explicit class-factory versions before reference execution.'
p=S/'classmode-manifest.json';p.write_text(json.dumps(m,indent=2)+'\n');p.chmod(0o444);b=gzip.compress(p.read_bytes(),mtime=0);(S/'classmode-manifest.json.gz').write_bytes(b);print(len(out),p,hashlib.sha256(p.read_bytes()).hexdigest(),len(p.read_bytes()));print('gzip',hashlib.sha256(b).hexdigest(),len(b))
