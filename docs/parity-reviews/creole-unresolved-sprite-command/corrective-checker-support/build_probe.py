# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,subprocess,hashlib,json,shutil,time
S=pathlib.Path(__file__).parent;R=pathlib.Path.cwd();D=S/'target-after/release/deps';sha=lambda b:hashlib.sha256(b).hexdigest()
shutil.copy2(S/'target-after/release/rustuml',S/'after-rustuml')
args=['rustc','--edition=2024',str(S/'probe.rs'),'-L','dependency='+str(D),'--extern','rustuml_oracle='+str(next(D.glob('librustuml_oracle-*.rlib'))),'--extern','rustuml_parser='+str(next(D.glob('librustuml_parser-*.rlib'))),'-o',str(S/'probe')]
p=subprocess.run(args,capture_output=True);(S/'logs/probe.stdout').write_bytes(p.stdout);(S/'logs/probe.stderr').write_bytes(p.stderr)
rows=[]
for phase,rev,cmd in [('before','246ff6396b538843143ee9d2cc9a569b67f614dd',['cargo','build','-p','rustuml','--release','-j','4']),('after','a84856637adcd8e8d4e78fd2ee00658c9c612feb',['cargo','build','--workspace','--release','-j','4'])]:
 rows.append(dict(phase=phase,source_revision=rev,source_tree=subprocess.check_output(['git','rev-parse',rev+'^{tree}'],text=True).strip(),binary=str(S/(phase+'-rustuml')),binary_sha256=sha((S/(phase+'-rustuml')).read_bytes()),argv=cmd,cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4'),exit=0,log='logs/build-'+phase+'.log',source_hashes={str(f):sha(f.read_bytes()) if phase=='after' else sha(subprocess.check_output(['git','show',rev+':'+str(f)])) for f in [pathlib.Path('crates/rustuml-render/src/'+n+'.rs') for n in ['creole','text_render','wbs','class','sequence']]}))
(S/'build-provenance.json').write_text(json.dumps(dict(builds=rows,probe=dict(argv=args,exit=p.returncode,source_sha256=sha((S/'probe.rs').read_bytes()),binary_sha256=sha((S/'probe').read_bytes())),toolchain=subprocess.check_output(['rustc','-Vv'],text=True)),indent=2)+'\n')
assert p.returncode==0
