# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import subprocess,os,json,hashlib
from pathlib import Path
S=Path(__file__).parent;T=S/'target-before';env=dict(os.environ,CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='4');cmd=['cargo','build','-p','rustuml-oracle','--release','-j','4']
with (S/'logs/build-probe.log').open('wb') as f:p=subprocess.run(cmd,cwd=S/'source-before',env=env,stdout=f,stderr=subprocess.STDOUT)
assert p.returncode==0
args=['rustc','--edition=2024',str(S/'probe.rs'),'-L','dependency='+str(T/'release/deps'),'--extern','rustuml_oracle='+str(T/'release/librustuml_oracle.rlib'),'--extern','rustuml_parser='+str(next((T/'release/deps').glob('librustuml_parser-*.rlib'))),'-o',str(S/'probe')]
p=subprocess.run(args,capture_output=True);(S/'logs/rustc-probe.stdout').write_bytes(p.stdout);(S/'logs/rustc-probe.stderr').write_bytes(p.stderr);assert p.returncode==0,p.stderr
(S/'diagnostic-build.json').write_text(json.dumps(dict(cargo=cmd,rustc=args,exit=p.returncode,source_sha256=hashlib.sha256((S/'probe.rs').read_bytes()).hexdigest(),binary_sha256=hashlib.sha256((S/'probe').read_bytes()).hexdigest()),indent=2)+'\n');print('Diagnostic built')
