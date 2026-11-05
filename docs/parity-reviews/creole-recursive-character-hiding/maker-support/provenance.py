# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd,text=True).strip()
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
base='a3afe487a66ae26c73e2f1853d6c26fc2d228875';prod=git('rev-parse','a6e1718c4');jrev='71806a23780b04a5ccde2f8ceb5121edad5eb711';assert git('rev-parse','HEAD',cwd=J)==jrev
prior=json.loads((R/'docs/parity-reviews/creole-style-candidate-eligibility/maker-support/java-source-provenance.json').read_text())['files'];paths={r['path'] for r in prior}|{str(J/'src/main/java/net/sourceforge/plantuml'/p) for p in ['TitledDiagram.java','core/DiagramChromeFactory12026.java','warning/Warning.java','klimt/creole/legacy/AtomText.java']};rows=[]
for path in sorted(paths):
 p=Path(path);b=p.read_bytes();assert b==subprocess.check_output(['git','show',jrev+':'+str(p.relative_to(J))],cwd=J);rows.append(dict(path=path,sha256=sha(b),matches_pin=True))
dump('java-source-provenance.json',dict(revision=jrev,files=rows,execution='Parent existing Monkey Man only; Java argv derived, not process-traced. No historical font/Graphviz equivalence or math claims.'))
files=['crates/rustuml-render/src/creole.rs','crates/rustuml-render/src/text_render.rs','crates/rustuml-render/src/wbs.rs','crates/rustuml-parser/src/diagram/wbs.rs','crates/rustuml-parser/src/parse/wbs.rs','crates/rustuml-render/src/class.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-parser/src/parse/class.rs','crates/rustuml-parser/src/preprocess/mod.rs'];assert git('diff','--name-only',base,prod,'--','crates').splitlines()==files[:1]
builds=[]
for phase,rev in [('before',git('rev-parse','f75103e50')),('after',prod)]:
 hashes={p:sha(subprocess.check_output(['git','show',rev+':'+p])) for p in files};builds.append(dict(phase=phase,revision=rev,source_tree=git('rev-parse',rev+'^{tree}'),source_hashes=hashes,binary=str(S/(phase+'-rustuml')),binary_sha256=sha((S/(phase+'-rustuml')).read_bytes()),argv=['cargo','build','-p','rustuml','--release','-j','4'],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4'),exit=0,log='logs/build-'+phase+'.log',status_provenance='Captured successful Finished release log. Separate private target seeded with APFS clone; no shell worktree.'))
for p in files:assert builds[0]['source_hashes'][p]==sha(subprocess.check_output(['git','show',base+':'+p]));assert builds[1]['source_hashes'][p]==sha((R/p).read_bytes())
before=subprocess.check_output(['git','show',base+':'+files[0]]);after=(R/files[0]).read_bytes();assert before[:before.index(b'pub struct Style')]==after[:after.index(b'pub struct Style')]
dump('build-provenance.json',dict(base=base,production=prod,builds=builds,only_production_file=files[0],legacy_prefix_unchanged=True,public_shared_signatures_unchanged=True,toolchain=dict(rustc=subprocess.check_output(['rustc','-Vv'],text=True),cargo=subprocess.check_output(['cargo','-V'],text=True)),diagnostic=dict(source_sha256=sha((S/'probe.rs').read_bytes()),binary_sha256=sha((S/'probe').read_bytes()),origin='Unchanged style/WBS maker diagnostic using unchanged compare_svg_strict')))
checks=[]
for name,want in [('reference-results.json.gz','3db82bcd6e971d1b52340d8104271bce28059cea3493f46b35eabb6f779a3cda'),('reference-transport.json','e2f8d4376537b07a508b43b0a221d8dc043beb59ae3c507ec9af70a524d246b9'),('parent-relay-input.json.gz','4f37c64fd1a9664f38ff1712237ce04cfd027a93372f8a4d279e841312abace8'),('request.json','f3ffa40b8bc1b1ccdca109dc7037db0afb46ca17a6c38b9f13f8954457de3419'),('manifest.json','693707e44fa05d70ec48f1063a61f2ddb967e42217a1fd3bdf129f79aff01351'),('input-bundle.json.gz','958158ca630045f0dfb3f917a67c2e2319699c1e701097d6861631b2bd420714')]:
 assert sha((S/name).read_bytes())==want;checks.append(dict(path=name,sha256=want))
dump('relay-envelope-hashes.json',checks)
dump('focused-results.json',[dict(argv=['cargo','test','-p','rustuml-render','--lib','--release','creole::'],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4'),log='logs/focused-creole.log',exit=101,passed=86,failed=1,note='New test incorrectly expected nested size tags to match balanced closes. Java uses first closing match, not a balanced parser. Production was not changed to fit this expectation.'),dict(argv=['cargo','test','-p','rustuml-render','--lib','--release','creole::'],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4'),log='logs/focused-creole-corrected.log',exit=0,passed=87,failed=0,note='Use source-defined nested EOL size candidates to isolate equal-style flush boundaries. Previous failed test retained in edit.py and log.')])
# Preserve every base-tracked historical evidence file, not just selected inventories.
paths=git('ls-tree','-r','--name-only',base,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();old=[]
for p in paths:
 b=(R/p).read_bytes();assert b==subprocess.check_output(['git','show',base+':'+p]);old.append(dict(path=p,sha256=sha(b)))
dump('base-evidence-integrity.json',dict(base=base,files=len(old),all_equal=True,inventory=old));print('preserved base evidence',len(old),flush=True)
