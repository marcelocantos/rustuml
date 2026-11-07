# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,shutil,subprocess,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();D=C/'intermediate-size-ba3d';D.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
p=D/'rustuml'
if not p.exists():shutil.copy2('/Users/marcelo/Library/Caches/rustuml-sequence-repair/target/release/rustuml',p)
rows=[]
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
for rev,bin in [('before',C/'before-bin/rustuml'),('bad_size_guard',p),('restored',C/'final-bin/rustuml')]:
 for item in json.loads((C/'size8.json').read_text()):
  s=Path(item['source']);b=s.read_bytes();rec=dict(revision=rev,binary_sha256=sha(bin.read_bytes()),source=item['source'],source_sha256=sha(b),results={})
  for mode,flags in [('svg',['-tsvg','-pipe']),('model',['--ast','-pipe']),('ascii',['-ttxt','-pipe']),('yaml',['--yaml','-pipe'])]:
   argv=[str(bin),*flags];r=subprocess.run(argv,input=b,capture_output=True,cwd=R,env=env,timeout=30)
   rec['results'][mode]=dict(argv=argv,cwd=str(R),exit=r.returncode,stdout=r.stdout.decode(),stderr=r.stderr.decode(),stdout_sha256=sha(r.stdout))
   if mode=='svg':
    out=D/(rev+'-'+s.stem+'.svg');out.write_bytes(r.stdout)
    compare=subprocess.run([str(C/'before-bin/compare_cli'),item['golden'],str(out)],capture_output=True)
    rec['comparison']=dict(exit=compare.returncode,stdout=compare.stdout.decode(),stderr=compare.stderr.decode());rec['family']=ET.fromstring(r.stdout).attrib.get('data-diagram-type') if r.returncode==0 else None
  rows.append(rec)
(C/'size-diagnostics.json').write_text(json.dumps(rows,indent=2)+'\n')
print([(x['revision'],Path(x['source']).stem,x['results']['svg']['exit'],x['comparison']['exit'],x['family']) for x in rows])
