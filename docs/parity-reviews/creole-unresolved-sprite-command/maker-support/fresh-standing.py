# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,shutil,subprocess,time
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-maker');R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45');dep=S/'target-after/release/deps'
sha=lambda b:hashlib.sha256(b).hexdigest()
source=S/'standing_slice.rs';shutil.copy2('/Users/marcelo/Library/Caches/rustuml-tim8-checker/standing_slice.rs',source)
cmd=['rustc','--edition=2024',str(source),'-L','dependency='+str(dep)];linked={}
for crate in ['rustuml_parser','rustuml_oracle','rustuml_render']:
 p=max(dep.glob('lib'+crate+'-*.rlib'),key=lambda p:p.stat().st_mtime_ns);cmd+=['--extern',crate+'='+str(p)];linked[str(p)]=sha(p.read_bytes())
cmd+=['-o',str(S/'standing-slice')];q=subprocess.run(cmd,capture_output=True);(S/'logs/standing-slice-build.stdout').write_bytes(q.stdout);(S/'logs/standing-slice-build.stderr').write_bytes(q.stderr)
(S/'standing-slice-build.json').write_text(json.dumps(dict(argv=cmd,cwd=str(R),exit=q.returncode,source_sha256=sha(source.read_bytes()),linked=linked),indent=2)+'\n');assert q.returncode==0
out=[];env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RAYON_NUM_THREADS='4')
for row in json.loads((S/'new-manifest.json').read_bytes()):
 p=Path(row['source']);g=S/'fresh-reference'/p.stem/'java.svg.stdout';d=S/'fresh-standing'/p.stem;d.mkdir(parents=True,exist_ok=True);cmd=[str(S/'standing-slice'),str(p),str(g)];q=subprocess.run(cmd,cwd=R/'crates/rustuml-oracle',env=env,capture_output=True)
 (d/'stdout').write_bytes(q.stdout);(d/'stderr').write_bytes(q.stderr);out.append(dict(source=str(p),reference=str(g),argv=cmd,cwd=str(R/'crates/rustuml-oracle'),exit=q.returncode,stdout_sha256=sha(q.stdout),stderr_sha256=sha(q.stderr)))
(S/'fresh-standing.json').write_text(json.dumps(out,indent=2)+'\n');print(sum(x['exit']==0 for x in out),'/',len(out),'fresh unchanged standing-algorithm matches')
