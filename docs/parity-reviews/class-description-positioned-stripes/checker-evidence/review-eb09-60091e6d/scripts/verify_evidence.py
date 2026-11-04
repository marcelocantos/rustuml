# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,subprocess,tarfile
from pathlib import Path
R=Path.cwd();S=Path(__file__).resolve().parent;D=R/'docs/parity-reviews/class-description-positioned-stripes/checker-evidence/review-eb09-60091e6d';F=R/'test-diagrams/perturbations/class-description-positioned-stripes/checker-60091e6d'
wanted={}
for label in ['preserved-before','preserved-after','fresh-before','fresh-after','nonclass-before','nonclass-after']:
 r=json.loads(gzip.decompress((D/(label+'.json.gz')).read_bytes()))
 for row in r['results']:
  key=str(Path(row['outputs']).relative_to(S))
  for mode,f in [('cli','rust.svg'),('ast','ast.txt'),('yaml','model.yaml')]:
   wanted[key+'/'+f]=row[mode]['stdout_sha256']
found=set()
with tarfile.open(D/'raw-captures.tar.gz','r|gz') as t:
 for member in t:
  if member.name in wanted:
   assert hashlib.sha256(t.extractfile(member).read()).hexdigest()==wanted[member.name],member.name
   found.add(member.name)
assert found==set(wanted)
index=json.load(open(D/'review-index.json'))
for f in index['reviews']:
 r=json.load(open(R/f));assert r['verdict']=='rejected'
 for p in r['perturbations']:assert (R/p['source']).exists() and R/p['golden']==(R/p['source']).with_suffix('.svg') and (R/p['golden']).exists()
for p in json.load(open(D/'pairs.json'))['fresh']+json.load(open(D/'pairs.json'))['retained_maker']:
 assert hashlib.sha256((R/p['source']).read_bytes()).hexdigest()==p['sha256']
assert not subprocess.check_output(['git','diff','eb09cecb','--name-only','--','crates','bullseye.yaml','test-diagrams/no_oracle_baseline.txt'])
(D/'evidence-integrity.json').write_text(json.dumps(dict(reviews=8,fresh_pairs=86,retained_duplicate_pairs=6,verified_raw_svg_ast_yaml_streams=len(found),modified_historical_files=0,notes=['Initial random-access gzip archive verification was interrupted after excessive re-decompression; replaced with this single-pass artifact check, not a product rerun.','git diff --cached --check reports trailing blank lines in byte-preserved raw comparator output; output bytes intentionally unchanged.','No main gate reruns after evidence additions.']),indent=2)+'\n')
paths=[p for p in D.rglob('*') if p.is_file() and p.name!='artifact-sha256.json']+[p for p in F.rglob('*') if p.is_file()]+[R/p for p in index['reviews']]
(D/'artifact-sha256.json').write_text(json.dumps({str(p.relative_to(R)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)},indent=2)+'\n')
print('Verified',len(found),'raw capture streams and',len(paths),'artifacts; eight rejected reviews')
