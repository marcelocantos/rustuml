# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess,gzip,base64
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');sha=lambda b:hashlib.sha256(b).hexdigest();base='bdfeba117c7af6a527fc66b0acda6d8273fa6d74';prod='1ffb930b7b56decb15208121db5e70d252ed3c1b';jrev='71806a23780b04a5ccde2f8ceb5121edad5eb711'
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd,text=True).strip()
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
assert git('rev-parse','HEAD',cwd=J)==jrev
prior=json.loads((R/'docs/parity-reviews/creole-parameter-candidate-grammar/maker-support/java-source-provenance.json').read_bytes())['files'];paths={r['path'] for r in prior}|{str(J/'src/main/java/net/sourceforge/plantuml'/p) for p in ['klimt/creole/atom/AtomImg.java','klimt/color/ColorTrieNode.java','klimt/color/HColor.java','klimt/awt/XColor.java','klimt/drawing/svg/DriverTextSvg.java','UgDiagram.java','crash/CrashReportHandler.java','klimt/creole/command/CommandCreoleStyle.java']};rows=[]
for path in sorted(paths):
 p=Path(path);b=p.read_bytes();assert b==subprocess.check_output(['git','show',jrev+':'+str(p.relative_to(J))],cwd=J);rows.append(dict(path=path,sha256=sha(b),matches_pin=True))
dump('java-source-provenance.json',dict(revision=jrev,files=rows,execution='Parent existing Monkey Man only; Java argv derived, not process-traced. No historical font/Graphviz equivalence or math claims.'))
files=['crates/rustuml-render/src/creole.rs','crates/rustuml-render/src/text_render.rs','crates/rustuml-render/src/wbs.rs','crates/rustuml-parser/src/diagram/wbs.rs','crates/rustuml-parser/src/parse/wbs.rs','crates/rustuml-render/src/class.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-parser/src/parse/class.rs','crates/rustuml-parser/src/preprocess/mod.rs'];assert git('diff','--name-only',base,prod,'--','crates').splitlines()==files[:1]
builds=[]
for phase,rev in [('before','f977806cef5a90a868175e8e707a89d4d791d29d'),('after',prod)]:
 hashes={p:sha(subprocess.check_output(['git','show',rev+':'+p])) for p in files};builds.append(dict(phase=phase,revision=rev,source_tree=git('rev-parse',rev+'^{tree}'),source_hashes=hashes,binary=str(S/(phase+'-rustuml')),binary_sha256=sha((S/(phase+'-rustuml')).read_bytes()),argv=['cargo','build','-p','rustuml','--release','-j','4'],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4'),exit=0,log='logs/build-'+phase+'.log',status_provenance='Captured successful Finished release log. Separate private target seeded with APFS clone; no shell worktree.'))
for p in files:assert builds[0]['source_hashes'][p]==sha(subprocess.check_output(['git','show',base+':'+p]));assert builds[1]['source_hashes'][p]==sha((R/p).read_bytes())
before=subprocess.check_output(['git','show',base+':'+files[0]]);after=(R/files[0]).read_bytes();assert before[:before.index(b'pub struct Style')]==after[:after.index(b'pub struct Style')]
for start,end in [(b'pub struct Style',b'pub fn parse_segments'),(b'fn parameter_command',b'fn walk_parameter_body')]:
 x=before[before.index(start):before.index(end)];y=after[after.index(start):after.index(end)]
 if start==b'fn parameter_command':y=y[:y.index(b'// HColorSet.parseColor')]
 assert x.rstrip()==y.rstrip()
dump('build-provenance.json',dict(base=base,production=prod,builds=builds,only_production_file=files[0],legacy_prefix_unchanged=True,public_style_and_shared_signatures_unchanged=True,candidate_grammars_unchanged=True,toolchain=dict(rustc=subprocess.check_output(['rustc','-Vv'],text=True),cargo=subprocess.check_output(['cargo','-V'],text=True)),diagnostic=dict(source_sha256=sha((S/'probe.rs').read_bytes()),binary_sha256=sha((S/'probe').read_bytes()),origin='Unchanged prior maker diagnostic using unchanged compare_svg_strict')))
checks=[]
for name,want in [('reference-results.json.gz','6f76972eead6770aede07d7bdcf46c61a50d6354cea834545504367176527d20'),('reference-transport.json','e619c22fa9b52c0ceae3c64cb45c90db0a99ddaf6bbbb17279a3943fbeaea554'),('parent-relay-input.json.gz','01e113dbb82139b50d10091f8b25f03a6ccf3056998cd286b4cc2d577709d7a7'),('request.json','1dbd085c870457b51fe0ebb4fc8d71de6ffb9241c8602e9ca49c3f3467ef613a'),('manifest.json','6f67b8684a5b1339562c178457217275fec697b97b88b5ef490e8bee23012819'),('input-bundle.json.gz','583c8cf20bafa6893d0ae23f7128fc977190400b95d1bf11217b33c9bb0f1c33')]:
 assert sha((S/name).read_bytes())==want;checks.append(dict(path=name,sha256=want))
dump('relay-envelope-hashes.json',checks)
transport=json.loads((S/'reference-transport.json').read_bytes());assert base64.b64decode(transport['data'],validate=True)==(S/'reference-results.json.gz').read_bytes();relay=json.loads(gzip.decompress((S/'parent-relay-input.json.gz').read_bytes()));original=json.loads(gzip.decompress((S/'input-bundle.json.gz').read_bytes()));assert base64.b64decode(relay['request_base64'],validate=True)==(S/'request.json').read_bytes();assert base64.b64decode(relay['manifest_base64'],validate=True)==(S/'manifest.json').read_bytes();assert relay['request']==original['request'] and relay['manifest']==original['manifest'];assert len(relay['resources'])==len(original['files'])==16
for a,b in zip(relay['resources'],original['files']):assert a['local_path']==b['local_path'] and a['remote_relative_path']==b['remote_relative_path'];assert base64.b64decode(a['data'],validate=True)==base64.b64decode(b['base64'],validate=True)
dump('relay-lossless-mapping-validation.json',dict(transport_bytes_equal=True,metadata_bytes_equal=True,resources=16,all_resource_bytes_equal=True))
dump('focused-results.json',[dict(argv=['cargo','test','-p','rustuml-render','--lib','--release','creole::'],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4'),log='logs/'+log,exit=status,passed=passed,failed=failed,note=note) for log,status,passed,failed,note in [('focused.log',101,0,0,'Compile error in new test: strip_markup misspelled existing stripped_text; test identifier corrected.'),('focused-corrected.log',101,91,3,'Two new tests incorrectly assumed balanced same-name closers, contrary to source first-close semantics. Existing image test expected inherited color; source blackBlueTrue requires explicit black. No production change for these test failures.'),('focused-final.log',0,94,0,'Use different outer font closer for nested color test; assert first-close background behavior; update image color expectation from named Java source. Alias resolver tests and public APIs unchanged.')]])
paths=git('ls-tree','-r','--name-only',base,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();old=[]
with subprocess.Popen(['git','cat-file','--batch'],stdin=subprocess.PIPE,stdout=subprocess.PIPE) as proc:
 for p in paths:
  proc.stdin.write((base+':'+p+'\n').encode());proc.stdin.flush();header=proc.stdout.readline().decode().split();assert header[1]=='blob';b=proc.stdout.read(int(header[2]));assert proc.stdout.read(1)==b'\n';assert b==(R/p).read_bytes();old.append(dict(path=p,sha256=sha(b)))
 proc.stdin.close();assert proc.wait()==0
dump('base-evidence-integrity.json',dict(base=base,files=len(old),all_equal=True,inventory=old));print('preserved base evidence',len(old),flush=True)
