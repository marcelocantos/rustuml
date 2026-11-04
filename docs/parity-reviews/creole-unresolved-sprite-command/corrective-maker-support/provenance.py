# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd,text=True).strip()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
assert git('rev-parse','HEAD',cwd=J)=='71806a23780b04a5ccde2f8ceb5121edad5eb711'
java=[('klimt/creole/legacy/StripeSimple.java',[124,150,218,275,288,295]),('klimt/creole/legacy/CreoleStripeSimpleParser.java',[160]),('klimt/creole/legacy/CreoleParser.java',[175]),('klimt/creole/Display.java',[99,258,353,605]),('klimt/creole/command/CommandCreoleSprite.java',[40]),('klimt/creole/command/CommandCreoleSizeChange.java',[40]),('klimt/creole/command/CommandCreoleStyle.java',[40]),('klimt/creole/command/Splitter.java',[40]),('klimt/creole/legacy/AtomText.java',[150]),('klimt/creole/SheetBlock1.java',[78,196,207]),('wbs/WBSTextBlock.java',[87]),('wbs/ITFLeaf.java',[58,75]),('wbs/ITFComposed.java',[60]),('wbs/Fork.java',[60]),('activitydiagram3/ftile/vertical/FtileBoxOld.java',[134,145,180,225]),('utils/CharHidder.java',[40]),('text/Guillemet.java',[76]),('klimt/font/FontStyle.java',[81]),('command/NameAndCodeParser.java',[48]),('classdiagram/command/CommandCreateClass.java',[127,163])]
source_rows=[]
for relative,locations in java:
 p=J/'src/main/java/net/sourceforge/plantuml'/relative;source_rows.append(dict(path=str(p),sha256=sha(p.read_bytes()),locations=locations))
p=J/'src/main/resources/skin/plantuml.skin';source_rows.append(dict(path=str(p),sha256=sha(p.read_bytes()),locations=[339,340,344]))
dump(S/'java-source-provenance.json',dict(revision=git('rev-parse','HEAD',cwd=J),files=source_rows,execution='No local Java/javac/SSH or runner created; only parent existing relay. Source citations identify pinned source; not a global historical font/Graphviz equivalence claim.'))
files=['crates/rustuml-render/src/creole.rs','crates/rustuml-render/src/text_render.rs','crates/rustuml-render/src/wbs.rs','crates/rustuml-render/src/class.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-parser/src/parse/class.rs','crates/rustuml-parser/src/preprocess/mod.rs']
rows=[]
for phase,revision in [('before','6b0ca33e88127a44bad1e564544db7d746816d6d'),('after',git('rev-parse','59dc6060'))]:
 binary=S/(phase+'-rustuml');rows.append(dict(phase=phase,source_revision=revision,source_tree=git('rev-parse',revision+'^{tree}'),binary=str(binary),binary_sha256=sha(binary.read_bytes()),argv=['cargo','build','-p','rustuml','--release','-j','4'],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4'),exit=0,log='logs/build-before.log' if phase=='before' else 'logs/build-after-initial.log',status_provenance='Successful shell build and captured Finished release log; no retroactively invented timing',source_hashes={f:sha(subprocess.check_output(['git','show',revision+':'+f],cwd=R)) for f in files}))
assert all(sha(subprocess.check_output(['git','show','246ff6396b538843143ee9d2cc9a569b67f614dd:'+f],cwd=R))==rows[0]['source_hashes'][f] for f in files)
dump(S/'build-provenance.json',dict(builds=rows,toolchain=dict(rustc=subprocess.check_output(['rustc','-Vv'],text=True),cargo=subprocess.check_output(['cargo','-V'],text=True)),diagnostics={n:dict(source=str(S/(n+'.rs')),source_sha256=sha((S/(n+'.rs')).read_bytes()),binary_sha256=sha((S/({'source_probe':'source-probe'}.get(n,n))).read_bytes()),provenance='Unchanged diagnostic copied from rustuml-unresolved-sprite-checker; original build record retained at checker-support/diagnostic-builds.json') for n in ['probe','source_probe']}))
dump(S/'focused-commands.json',[dict(argv=['cargo','test','-p','rustuml-render','--lib','--release',selector],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4'),exit=0,log='logs/'+log,status_provenance='Captured passing test-result line') for selector,log in [('creole::','focused-creole-initial.log'),('present_empty_lines','focused-empty.log'),('unknown_angle_candidates','focused-traversal.log'),('text_render::','focused-text-render.log'),('wbs::','focused-wbs.log')]])
print('recorded sources and binaries')
