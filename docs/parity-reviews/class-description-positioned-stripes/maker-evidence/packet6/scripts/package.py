# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-class-cff6');D=Path('docs/parity-reviews/class-description-positioned-stripes/maker-evidence/packet6');D.mkdir(parents=True,exist_ok=True)
# Preserve returned transport bytes and every fresh source/reference/result.
for name in ['reference-results.json','reference-results.json.gz','reference-transport.json','reference-validation.json','fresh-manifest.json','fresh-inputs.json.gz','fresh-transport.json','cohorts.json','deltas-and-residuals.json','model-retention.json','gate-delta.json','corrected-gates.json','all1649.json','review34.json','fresh26.json']:
 shutil.copy2(S/name,D/name)
for name in ['final1649','final26','before26','before34','after34-v1','after34-v2','after34-v3','after34-v4','old56-v3','old56-v4','descriptions-v3','descriptions-v4','fresh26-v3','fresh26-v4']:
 (D/(name+'.json.gz')).write_bytes(gzip.compress((S/(name+'.json')).read_bytes(),mtime=0))
F=D/'fresh-pairs';F.mkdir(exist_ok=True)
for p in sorted((S/'fresh-sources').glob('*.puml')):
 for suffix in ['.puml','.svg','.java.stderr']:
  shutil.copy2(p.with_suffix(suffix),F/p.with_suffix(suffix).name)
 for variant,folder in [('before','before26'),('after','final26')]:
  for suffix,new in [('.rust.svg',f'.{variant}.svg'),('.diff.txt',f'.{variant}.diff.txt'),('.rust.svg.stderr',f'.{variant}.stderr')]:
   src=S/folder/'fresh'/p.with_suffix(suffix).name;shutil.copy2(src,F/p.with_suffix(new).name)
  shutil.copy2(S/'models'/(p.stem+f'.{variant}.yaml'),F/(p.stem+f'.{variant}.yaml'))
A=D/'cli-iterations-models-and-gates.tar.gz'
with tarfile.open(A,'w:gz') as tar:
 for name in ['final1649','before34','after34-v1','after34-v2','after34-v3','after34-v4','old56-v3','old56-v4','descriptions-v3','descriptions-v4','fresh26-v3','fresh26-v4','models']:
  tar.add(S/name,arcname=name)
 for p in sorted(S.glob('*.log')):tar.add(p,arcname='logs/'+p.name)
 for p in sorted(S.glob('trace*.svg')):tar.add(p,arcname='diagnostics/'+p.name)
 for name in ['golden_failure_names.txt','no_oracle_failure_names.txt','golden_ulp_candidates.txt']:
  tar.add(S/name,arcname=name)
 # Exact temporary instrumentation/edit scripts preserve how intermediate
 # outputs were obtained; they are not production or a review attestation.
 for p in sorted(S.glob('*.py')):
  if p.name!='package.py':tar.add(p,arcname='scripts/'+p.name)
for name in ['replay.py','final_gates.py','models.py','summarize.py','unpack_refs.py','package.py']:
 p=D/'scripts'/name;p.parent.mkdir(exist_ok=True);shutil.copy2(S/name,p)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
prod=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
commits=[]
for line in subprocess.check_output(['git','log','--format=%H %s','b074d208..HEAD'],text=True).splitlines():
 h,title=line.split(' ',1);commits.append({'commit':h,'subject':title,'changed_files':subprocess.check_output(['git','diff-tree','--no-commit-id','--name-only','-r',h],text=True).splitlines()})
manifest={'production_revision':prod,'baseline_revision':'b074d2082cb9ed2b9460f79a32b170e31cc1ab5e','imported_evidence_original':'5416610690a5b507b6fd2e95a0cd30f780bd1192','commits':commits,'production_files':subprocess.check_output(['git','diff-tree','--no-commit-id','--name-only','-r',prod],text=True).splitlines(),'binary':str(S/'final-rustuml'),'binary_sha256':sha(S/'final-rustuml'),'baseline_binary_sha256':sha(Path('/Users/marcelo/Library/Caches/rustuml-class-cff5/final-rustuml')),'comparator_sha256':sha(Path('/Users/marcelo/Library/Caches/rustuml-class-cff4/check_svg')),'comparator_source':'docs/parity-reviews/class-command-grammar-closure/checker-support/scripts/check_svg.rs','comparator_source_sha256':sha(Path('docs/parity-reviews/class-command-grammar-closure/checker-support/scripts/check_svg.rs')),'reference_environment':json.load(open(S/'reference-results.json'))['environment'],'artifacts':{str(p.relative_to(D)):sha(p) for p in sorted(D.rglob('*')) if p.is_file()}}
(D/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('packaged',len(manifest['artifacts']),'files',sum(p.stat().st_size for p in D.rglob('*') if p.is_file()))
