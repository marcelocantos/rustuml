# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Reproduction of the recovery checker's local native probes; no Java needed.

Set ROOT to the candidate checkout and S to an extracted replay archive/scratch.
The recorded run used the equivalent inline Python commands. This packaged
script reproduces those operations; it was not used to regenerate Java SVGs.
"""
import json, subprocess
from pathlib import Path
S=Path('/tmp/rustuml-review-cbd')
lib=next((S/'target/release/build').glob('rustuml-layout-*/out/libgraphviz.a'))
command=['clang','-g','-O1','-fsanitize=address,undefined','-fno-omit-frame-pointer','-Ivendor/graphviz/include']
for p in ['', 'common','gvc','cgraph','pathplan','pack','cdt','label']:
    command+=['-Ivendor/graphviz/lib/'+p]
command += [str(S/'native_safety.c'),str(lib),'-o',str(S/'native_safety')]
subprocess.run(command,check=True)
records=[]
for index in range(11):
    cmd=[str(S/'native_safety'),str(index),str(S/'allocation-control.html')]
    p=subprocess.run(cmd,capture_output=True)
    records.append(dict(command=cmd,exit=p.returncode,stderr=p.stderr.decode(errors='replace')))
(S/'allocation-results.json').write_text(json.dumps(records,indent=2)+'\n')
records=[]
for path in sorted((S/'native').glob('*.html')):
    cmd=[str(S/'native_safety'),'0',str(path)]
    p=subprocess.run(cmd,capture_output=True,timeout=30)
    records.append(dict(name=path.stem,command=cmd,exit=p.returncode,stderr=p.stderr.decode(errors='replace')))
(S/'native-safety-results.json').write_text(json.dumps(records,indent=2)+'\n')
for path in sorted((S/'native').glob('*.dot')):
    for kind,cmd in [('native',[str(S/'raw_dot'),str(path)]),('local16',['/opt/homebrew/bin/dot','-Tplain',str(path)])]:
        p=subprocess.run(cmd,capture_output=True,timeout=30)
        path.with_suffix('.'+kind+'.txt').write_bytes(p.stdout)
        path.with_suffix('.'+kind+'.stderr').write_bytes(p.stderr)
