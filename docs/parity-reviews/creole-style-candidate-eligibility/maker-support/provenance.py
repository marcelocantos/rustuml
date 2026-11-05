# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd,text=True).strip()
def dump(name,v):(S/name).write_text(json.dumps(v,indent=2)+'\n')
base='a84856637adcd8e8d4e78fd2ee00658c9c612feb';candidate=git('rev-parse','8df6bb53f');jrev='71806a23780b04a5ccde2f8ceb5121edad5eb711'
assert git('rev-parse','HEAD',cwd=J)==jrev
prior=json.loads((R/'docs/parity-reviews/creole-unresolved-sprite-command/corrective-checker-support/audit-summary.json').read_text())['java_sources']
extra=json.loads((R/'docs/parity-reviews/creole-unresolved-sprite-command/corrective-checker-support/additional-java-source-audit.json').read_text())
paths={r['path'] for r in prior+extra}|{str(J/'src/main/java/net/sourceforge/plantuml'/p) for p in ['wbs/CommandWBSItemNew.java','klimt/shape/TextBlockUtils.java','klimt/creole/command/CommandCreoleColorChange.java']}
java=[]
for path in sorted(paths):
 p=Path(path);b=p.read_bytes();assert b==subprocess.check_output(['git','show',jrev+':'+str(p.relative_to(J))],cwd=J);java.append(dict(path=path,sha256=sha(b),matches_pin=True))
dump('java-source-provenance.json',dict(revision=jrev,files=java,execution='Parent existing Monkey Man only. Derived Java argv is not process-traced. No historical font/Graphviz environment equivalence claim.'))
files=git('diff','--name-only',base,candidate,'--','crates').splitlines();assert files==['crates/rustuml-parser/src/diagram/wbs.rs','crates/rustuml-parser/src/parse/wbs.rs','crates/rustuml-render/src/creole.rs','crates/rustuml-render/src/wbs.rs']
observed=files+['crates/rustuml-render/src/text_render.rs','crates/rustuml-render/src/class.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-parser/src/parse/class.rs','crates/rustuml-parser/src/preprocess/mod.rs']
builds=[]
for phase,revision,log in [('before',git('rev-parse','dc796484a'),'build-before.log'),('after',candidate,'build-after-delimiters-resumed.log')]:
 hashes={p:sha(subprocess.check_output(['git','show',revision+':'+p])) for p in observed}
 builds.append(dict(phase=phase,revision=revision,source_tree=git('rev-parse',revision+'^{tree}'),source_hashes=hashes,binary=str(S/(phase+'-rustuml')),binary_sha256=sha((S/(phase+'-rustuml')).read_bytes()),argv=['cargo','build','-p','rustuml','--release','-j','4'],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4'),exit=0,log='logs/'+log,status_provenance='Captured successful Finished release log; interrupted earlier build separately retained.'))
# Production bytes did not change between replay/build and this commit.
for p in observed:assert sha((R/p).read_bytes())==builds[1]['source_hashes'][p]
for p in observed:assert sha(subprocess.check_output(['git','show',base+':'+p]))==builds[0]['source_hashes'][p]
legacy=lambda b:b.split('/// A text segment')[0] if b'/// A text segment' in b else b[:b.index('pub struct Style')]
# The legacy tspan implementation occupies the prefix before segment types.
before=subprocess.check_output(['git','show',base+':crates/rustuml-render/src/creole.rs']);after=(R/'crates/rustuml-render/src/creole.rs').read_bytes();boundary=before.index(b'pub struct Style');assert before[:boundary]==after[:after.index(b'pub struct Style')]
dump('build-provenance.json',dict(base=base,production=candidate,builds=builds,production_files=files,legacy_tspan_prefix_unchanged=True,public_shared_signatures='No public shared text signature changed; additive parser WbsShape/WbsNode.shape explicitly authorized.',toolchain=dict(rustc=subprocess.check_output(['rustc','-Vv'],text=True),cargo=subprocess.check_output(['cargo','-V'],text=True)),diagnostic=dict(source_sha256=sha((S/'probe.rs').read_bytes()),binary_sha256=sha((S/'probe').read_bytes()),provenance='Unchanged diagnostic from corrective checker cache, uses unchanged compare_svg_strict; no new comparator.')))
rows=[]
for log in sorted((S/'logs').glob('focused-*.log')):
 text=log.read_text();rows.append(dict(log=str(log.relative_to(S)),status='compile_error' if 'could not compile' in text else 'pass' if 'test result: ok.' in text else 'unknown',result_lines=[l for l in text.splitlines() if 'test result:' in l],note='Initial accidental edit in legacy arm failed to compile, restored before proper shared implementation; no bad code committed.' if log.name=='focused-creole-initial.log' else None))
dump('focused-results.json',rows)
transport=[]
for folder,want in [('', '3656d71ffb5770e8f3938dbd9a36d14f303af75a888d32fb65144837d22dc28e'),('delimiters','df112f76069ff366565c8487eed617b04e24ce6a5d8da09e757c624ef1875092'),('delimiters-focus','56bd77e5e83712c2d021242cdfd66e898797cc16cf07c052a13f42a4a880f152')]:
 d=S/folder;assert sha((d/'reference-transport.json').read_bytes())==want
 transport.append(dict(folder=folder,transport_sha256=want,validation=json.loads((d/'reference-validation.json').read_bytes()),classification=json.loads((d/'fresh-classification.json').read_bytes())))
dump('all-relay-validation.json',transport)
print(candidate,files)
