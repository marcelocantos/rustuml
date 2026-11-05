# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,shutil,tarfile
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/tim-builtin-value-boundaries/checker5-support';D.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
allrows=[]
for suite,remote in [('fresh24','relay24'),('supplement5','relay5')]:
 rows=json.loads((S/(suite+'-results.json')).read_bytes())
 for row in rows:
  p=Path(row['source']);dest=R/'test-diagrams/perturbations'/row['mechanism']/'checker5';dest.mkdir(parents=True,exist_ok=True)
  shutil.copy2(p,dest/p.name);shutil.copy2(S/remote/p.stem/'java.svg.stdout',dest/p.with_suffix('.svg').name)
  evidence=dest/'evidence'/p.stem;evidence.mkdir(parents=True,exist_ok=True)
  for f in (S/remote/p.stem).iterdir():shutil.copy2(f,evidence/f.name)
  for phase in ['before','after']:shutil.copytree(S/'replay'/suite/phase/row['capture_id'],evidence/phase,dirs_exist_ok=True)
  if (p.parent/'lib').exists() and row['mechanism']=='tim-sub-marker-layers':shutil.copytree(p.parent/'lib',dest/'lib',dirs_exist_ok=True)
  row['suite']=suite;row['committed_source']=str((dest/p.name).relative_to(R));row['committed_golden']=str((dest/p.with_suffix('.svg').name).relative_to(R));row['committed_evidence']=str(evidence.relative_to(R));allrows.append(row)
(D/'fresh-results.json').write_text(json.dumps(allrows,indent=2)+'\n')
raw=R/'test-diagrams/perturbations/tim-builtin-value-boundaries/checker5/relay';raw.mkdir(exist_ok=True)
for batch in ['tim24','tim5']:
 for suffix in ['-results.json.gz','-transport.json']:shutil.copy2(Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay')/(batch+suffix),raw/(batch+suffix))
 for suffix in ['-verification.json']:shutil.copy2(S/(batch+suffix),D/(batch+suffix))
for f in ['generate.py','generate_supplement.py','gates.py','integrity.py','replay.py','relay_verify.py','analyze.py','package.py','remote-request.json','new-manifest.json','supplement-request.json','supplement-manifest.json','integrity.json','auxiliary-integrity.json','gates.json','controls.json','analysis.json','dispatch-arity-audit.json','probe-build.json']:
 shutil.copy2(S/f,D/f)
for suite in ['collateral','prior31','maker15','maker17','fresh24','supplement5']:
 shutil.copy2(S/(suite+'-summary.json'),D/(suite+'-summary.json'))
 (D/(suite+'-results.json.gz')).write_bytes(gzip.compress((S/(suite+'-results.json')).read_bytes(),mtime=0))
(D/'source-models.json.gz').write_bytes(gzip.compress((S/'source-models.json').read_bytes(),mtime=0))
shutil.copy2(R/'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java',D/'SourceProbe.java')
with tarfile.open(D/'measurement-artifacts.tar.gz','w:gz') as tar:
 for n in ['replay','logs','controls']:tar.add(S/n,arcname=n)
summary=dict(candidate='1703a2727fa364ee8004ba59a22b4c6934ee5d3d',production='92647b126f6dc1c83ebdc6213d0c2c0a5b3211b6',before='ee0e4c72fb222f1312d1aaf7d5a152cc97f12781',accounts='6d46a483aad389d4535e1ba24822fdd116baf517',fresh_total=len(allrows),fresh_valid=sum(r['reference_valid'] for r in allrows),fresh_source_matches=sum(r['phases']['after'].get('source_equal',False) for r in allrows),fresh_svg_matches=sum(r['phases']['after']['svg_match'] for r in allrows),fresh_negative_rejected=sum(not r['reference_valid'] and r['phases']['after']['cli_exit']!=0 for r in allrows),fresh_valid_failures=[r['committed_source'] for r in allrows if r['reference_valid'] and not r['phases']['after']['svg_match']],fresh_lost_passes=[r['committed_source'] for r in allrows if r['phases']['before']['svg_match'] and not r['phases']['after']['svg_match']],artifact_sha256=sha((D/'measurement-artifacts.tar.gz').read_bytes()),local_java_launches=0,ssh_attempts=0,child_agents=0)
(D/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
