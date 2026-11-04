# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,gzip,shutil,tarfile,subprocess
from pathlib import Path
S=Path(__file__).resolve().parent;R=Path.cwd();D=R/'docs/parity-reviews/class-description-positioned-stripes/checker-evidence/review-eb09-60091e6d';D.mkdir(parents=True,exist_ok=True)
F=R/'test-diagrams/perturbations/class-description-positioned-stripes/checker-60091e6d';F.mkdir(parents=True,exist_ok=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
b=json.load(open(S/'fresh-before.json'))['results'];a=json.load(open(S/'fresh-after.json'))['results'];pairs=[]
for old,new in zip(b,a):
 src=Path(new['source']);out=F/src.name
 for ext in ['.puml','.svg','.java.stderr','.java.json']:shutil.copy2(src.with_suffix(ext),out.with_suffix(ext))
 status=dict(source_sha256=new['source_sha256'],java_sha256=new['java_sha256'],java=json.load(open(src.with_suffix('.java.json'))),before=old,after=new)
 out.with_suffix('.status.json').write_text(json.dumps(status,indent=2)+'\n')
 for label,row in [('before',old),('after',new)]:
  for orig,suffix in [('rust.svg',f'.{label}.svg'),('model.yaml',f'.{label}.yaml'),('diff.txt',f'.{label}.diff.txt')]:shutil.copy2(Path(row['outputs'])/orig,out.with_suffix(suffix))
 pairs.append(dict(source=str(out.relative_to(R)),golden=str(out.with_suffix('.svg').relative_to(R)),sha256=sha(out),kind='fresh',name=src.stem,status=new['status']))
P=R/'docs/parity-reviews/class-description-positioned-stripes/maker-evidence/packet6/fresh-pairs';retained=[]
preserved=json.load(open(S/'preserved-after.json'))['results']
for name in ['actor_blank','boundary_blank','actor_stereo','boundary_stereo','group_left','group_top']:
 source=P/(name+'.puml');dest=F/('retained_maker_'+name+'.puml')
 for ext in ['.puml','.svg','.java.stderr']:shutil.copy2(source.with_suffix(ext),dest.with_suffix(ext))
 row=next(x for x in preserved if (R/x['source']).resolve()==source)
 dest.with_suffix('.status.json').write_text(json.dumps(row,indent=2)+'\n');retained.append(dict(source=str(dest.relative_to(R)),golden=str(dest.with_suffix('.svg').relative_to(R)),original_source=str(source.relative_to(R)),sha256=sha(dest),kind='retained maker, duplicate enforcement copy; NOT fresh',name=name,status=row['status']))
(D/'pairs.json').write_text(json.dumps(dict(fresh=pairs,retained_maker=retained,denominators='86 fresh sources, all disjoint from preserved1675. Six maker copies duplicate existing source hashes; they add zero new controls or unique sources.'),indent=2)+'\n')
for name in ['audit.json','source-audit.json','java-source-provenance.json','gate-comparison.json','gates.json','replay-summary.json','fresh-observations.json','nonclass.json','fresh-manifest.json','context-manifest.json','classmode-manifest.json','reference-results-validation.json','context-reference-results-validation.json','classmode-reference-results-validation.json','all1649.json','review34.json']:
 shutil.copy2(S/name,D/name)
for name in ['preserved-before.json','preserved-after.json','fresh-before.json','fresh-after.json','nonclass-before.json','nonclass-after.json']:
 (D/(name+'.gz')).write_bytes(gzip.compress((S/name).read_bytes(),mtime=0))
for name in ['reference-results.json.gz','context-reference-results.json.gz','classmode-reference-results.json.gz','fresh-manifest.json.gz','context-manifest.json.gz','classmode-manifest.json.gz']:
 shutil.copy2(S/name,D/name)
with tarfile.open(D/'raw-captures.tar.gz','w:gz') as t:
 for folder in ['preserved-before','preserved-after','fresh-before','fresh-after','nonclass-before','nonclass-after','context-diagnostic']:
  t.add(S/folder,arcname=folder)
 for p in sorted(S.glob('*.log')):t.add(p,arcname='logs/'+p.name)
 for p in sorted(S.glob('*transport.json')):t.add(p,arcname='transport/'+p.name)
 for p in sorted(S.glob('*reference-results.json')):t.add(p,arcname='references/'+p.name)
 for name in ['golden_failure_names.txt','no_oracle_failure_names.txt']:t.add(S/name,arcname=name)
scriptdir=D/'scripts';scriptdir.mkdir(exist_ok=True)
for p in sorted(S.glob('*.py')):shutil.copy2(p,scriptdir/p.name)
identity=dict(reviewer_task='thread:delegated-task:command%3Amcp%3A7a6e5142-c330-4dbe-941b-87119bcc7cef%3Adelegate-task%3A39457d79-0e4b-4e29-88ed-087e0951d23d',reviewer_model='gpt-6-astra high / Codex harness / T3 Code',candidate='eb09cecbc90f5690c4a09bf49118f2feff8c1c12',production='d84b89cd8ee2ddd64ac71863004ec6ee84b1c4cd',before='b074d2082cb9ed2b9460f79a32b170e31cc1ab5e',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',java_jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',cwd=str(R),branch=subprocess.check_output(['git','branch','--show-current'],text=True).strip(),before_binary=str(S/'target-before/release/rustuml'),before_binary_sha256=sha(S/'target-before/release/rustuml'),after_binary=str(S/'target-after/release/rustuml'),after_binary_sha256=sha(S/'target-after/release/rustuml'),comparator_sha256=sha(S/'check_svg'),comparator_source='docs/parity-reviews/class-command-grammar-closure/checker-support/scripts/check_svg.rs',comparator_source_sha256=sha(R/'docs/parity-reviews/class-command-grammar-closure/checker-support/scripts/check_svg.rs'),rustc=subprocess.check_output(['rustc','-Vv'],text=True),before_build=dict(command=['cargo','build','--release','-p','rustuml'],exit=0,revision='b074d2082cb9ed2b9460f79a32b170e31cc1ab5e',environment={'CARGO_BUILD_JOBS':'4','CARGO_TARGET_DIR':str(S/'target-before')},log='raw-captures.tar.gz:logs/build-before.log'),policy='No children, worktrees, local Java/javac/SSH, native production/fault work, production/baseline/comparator/skip/target/policy edits or shipping. Evidence only. Main gates executed once before evidence additions; not rerun after references arrived. Existing parity_perturbations automatically collects new pairs; no acceptance or green standing-perturbation claim.')
(D/'identity.json').write_text(json.dumps(identity,indent=2)+'\n')
print('packaged',len(pairs),'fresh',len(retained),'retained duplicate maker pairs',D)
