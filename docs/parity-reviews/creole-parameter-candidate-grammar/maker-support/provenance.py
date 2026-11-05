# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd,text=True).strip()
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
base='2608469dace1f680a7e90c0895e44e657f8aaf7f';prod=git('rev-parse','8ca73318f');jrev='71806a23780b04a5ccde2f8ceb5121edad5eb711';assert git('rev-parse','HEAD',cwd=J)==jrev
prior=json.loads((R/'docs/parity-reviews/creole-style-candidate-eligibility/maker-support/java-source-provenance.json').read_text())['files'];paths={r['path'] for r in prior}|{str(J/'src/main/java/net/sourceforge/plantuml'/p) for p in ['TitledDiagram.java','core/DiagramChromeFactory12026.java','warning/Warning.java','klimt/creole/legacy/AtomText.java','klimt/creole/command/Splitter.java','klimt/creole/command/CommandCreoleColorAndSizeChange.java','klimt/creole/command/CommandCreoleSizeChange.java','klimt/creole/command/CommandCreoleColorChange.java','klimt/creole/command/CommandCreoleFontFamilyChange.java','klimt/creole/command/CommandCreoleImg.java','klimt/creole/command/CommandCreoleOpenIcon.java','regex/Pattern2.java','jaws/Jaws.java','klimt/color/HColorSet.java','klimt/drawing/svg/SvgGraphics.java','klimt/font/UFontImpl.java','klimt/font/FontStack.java','klimt/font/FontConfiguration.java']}|{str(J/'src/main/java/com/plantuml/ubrex/CharClassRaw.java')};rows=[]
for path in sorted(paths):
 p=Path(path);b=p.read_bytes();assert b==subprocess.check_output(['git','show',jrev+':'+str(p.relative_to(J))],cwd=J);rows.append(dict(path=path,sha256=sha(b),matches_pin=True))
dump('java-source-provenance.json',dict(revision=jrev,files=rows,execution='Parent existing Monkey Man only; Java argv derived, not process-traced. No historical font/Graphviz equivalence or math claims.'))
files=['crates/rustuml-render/src/creole.rs','crates/rustuml-render/src/text_render.rs','crates/rustuml-render/src/wbs.rs','crates/rustuml-parser/src/diagram/wbs.rs','crates/rustuml-parser/src/parse/wbs.rs','crates/rustuml-render/src/class.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-parser/src/parse/class.rs','crates/rustuml-parser/src/preprocess/mod.rs'];assert git('diff','--name-only',base,prod,'--','crates').splitlines()==files[:1]
builds=[]
for phase,rev in [('before',git('rev-parse','05683a971')),('after',prod)]:
 hashes={p:sha(subprocess.check_output(['git','show',rev+':'+p])) for p in files};builds.append(dict(phase=phase,revision=rev,source_tree=git('rev-parse',rev+'^{tree}'),source_hashes=hashes,binary=str(S/(phase+'-rustuml')),binary_sha256=sha((S/(phase+'-rustuml')).read_bytes()),argv=['cargo','build','-p','rustuml','--release','-j','4'],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4'),exit=0,log='logs/build-before.log' if phase=='before' else 'logs/build-after-alias.log',status_provenance='Captured successful Finished release log. Separate private target seeded with APFS clone; no shell worktree.'))
for p in files:assert builds[0]['source_hashes'][p]==sha(subprocess.check_output(['git','show',base+':'+p]));assert builds[1]['source_hashes'][p]==sha((R/p).read_bytes())
before=subprocess.check_output(['git','show',base+':'+files[0]]);after=(R/files[0]).read_bytes();assert before[:before.index(b'pub struct Style')]==after[:after.index(b'pub struct Style')]
dump('build-provenance.json',dict(base=base,production=prod,builds=builds,only_production_file=files[0],legacy_prefix_unchanged=True,public_shared_signatures_unchanged=True,toolchain=dict(rustc=subprocess.check_output(['rustc','-Vv'],text=True),cargo=subprocess.check_output(['cargo','-V'],text=True)),diagnostic=dict(source_sha256=sha((S/'probe.rs').read_bytes()),binary_sha256=sha((S/'probe').read_bytes()),origin='Unchanged style/WBS maker diagnostic using unchanged compare_svg_strict')))
checks=[]
for name,want in [('reference-results.json.gz','a3b538e932791d9fd6bd1c323f3f365a3376276b43cbc79d6ace094f2be74393'),('reference-transport.json','1a8a863e63a009a8be0f7a662f824b4c4184b324e2c3fdcdb2cd5d2345de9103'),('parent-relay-input.json.gz','8ee85a47546233248956e1e685201b6db1f111cdd70e842cff18c8369a37c4be'),('request.json','806899a9903fdbff9feec428efbb4d5f0fbbe82d9572109127172673914b5a13'),('manifest.json','550f1ade62d5a70aeffc368346fc177af0189387bb5dc1fbac2de311730f3ac9'),('input-bundle.json.gz','6ed6a2a11b796bdea72a3ccbf3558059c69dc9962e7acd6f45c3a057a883ce50')]:
 assert sha((S/name).read_bytes())==want;checks.append(dict(path=name,sha256=want))
dump('relay-envelope-hashes.json',checks)
dump('focused-results.json',[dict(argv=['cargo','test','-p','rustuml-render','--lib','--release','creole::'],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4'),log='logs/'+log,exit=status,passed=passed,failed=failed,note=note) for log,status,passed,failed,note in [('focused-initial.log',101,86,1,'Initial parameter draft introduced an icon barrier; existing segments_xml_escaping exposed changed empty-icon projection.'),('focused-grammar.log',101,89,1,'Three source-guided tests added; existing icon projection still failed. No existing expectation changed.'),('focused-final.log',0,90,0,'Restored existing icon empty handoff while retaining candidate validation. Production corrected, existing test unchanged. Initial draft, binary hash and all first-iteration replay outputs retained in iteration1.')]])
focused=json.loads((S/'focused-results.json').read_bytes())
focused.extend([dict(argv=['cargo','test','-p','rustuml-render','--lib','--release',filter],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4'),log='logs/'+log,exit=0,passed=count,failed=0,note='After narrow alias account5652ec397 and correction8ca73318f; existing consumer expectation unchanged.') for filter,log,count in [('creole::','focused-alias.log',91),('explicit_font_monospace_inherits_surrounding_metrics','focused-alias-consumer.log',1)]])
dump('focused-results.json',focused)
import base64,gzip
transport=json.loads((S/'reference-transport.json').read_bytes());assert base64.b64decode(transport['data'],validate=True)==(S/'reference-results.json.gz').read_bytes()
relay=json.loads(gzip.decompress((S/'parent-relay-input.json.gz').read_bytes()));original=json.loads(gzip.decompress((S/'input-bundle.json.gz').read_bytes()))
assert base64.b64decode(relay['request_base64'],validate=True)==(S/'request.json').read_bytes();assert base64.b64decode(relay['manifest_base64'],validate=True)==(S/'manifest.json').read_bytes()
assert relay['request']==original['request'] and relay['manifest']==original['manifest'];assert len(relay['resources'])==len(original['files'])==16
for a,b in zip(relay['resources'],original['files']):
 assert a['local_path']==b['local_path'] and a['remote_relative_path']==b['remote_relative_path'];assert base64.b64decode(a['data'],validate=True)==base64.b64decode(b['base64'],validate=True)
dump('relay-lossless-mapping-validation.json',dict(transport_bytes_equal=True,metadata_bytes_equal=True,resources=16,all_resource_bytes_equal=True))
# Preserve every base-tracked historical evidence file, not just selected inventories.
paths=git('ls-tree','-r','--name-only',base,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();old=[]
for p in paths:
 b=(R/p).read_bytes();assert b==subprocess.check_output(['git','show',base+':'+p]);old.append(dict(path=p,sha256=sha(b)))
dump('base-evidence-integrity.json',dict(base=base,files=len(old),all_equal=True,inventory=old));print('preserved base evidence',len(old),flush=True)
