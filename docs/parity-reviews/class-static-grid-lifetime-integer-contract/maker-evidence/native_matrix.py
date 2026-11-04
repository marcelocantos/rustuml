# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json, subprocess
from pathlib import Path
s=Path(__file__).resolve().parent
inputs=s/'native-inputs';inputs.mkdir(exist_ok=True)
prefix='<TABLE BORDER="0" CELLBORDER="0" CELLSPACING="0" CELLPADDING="0">'
def table(rows):
    return prefix+''.join('<TR>'+''.join(f'<TD FIXEDSIZE="TRUE" WIDTH="{w}" HEIGHT="{h}" PORT="p{r}_{c}"></TD>' for c,(w,h) in enumerate(row))+'</TR>' for r,row in enumerate(rows))+'</TABLE>'
shapes={'single':table([[(18,18)]]),'grid':table([[(0,7),(18,7),(2,7)],[(0,18),(18,18),(2,18)],[(0,3),(18,3),(2,3)]]),'ragged':table([[(18,18)],[(2,2),(3,3)]]),'partial':table([[(18,18),(3,3)]]).replace('</TABLE>','BROKEN')}
results=[]
def run(path,index):
    cmd=[str(s/'native_safety'),str(index),str(path)]
    p=subprocess.run(cmd,capture_output=True,text=True,timeout=20)
    r={'command':cmd,'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr}
    results.append(r)
    assert p.returncode in (0,10) and not p.stderr,r
    return json.loads(p.stdout)
for name,source in shapes.items():
    p=inputs/(name+'.html');p.write_text(source)
    base=run(p,0)
    for index in range(1,base['allocations']+2):
        r=run(p,index)
        if index<=base['allocations']: assert not r['supported'],(name,index,r)
# Retained independent controls: exercise the same bytes, no oracle regeneration.
for p in sorted((s/'checker-artifacts/native').glob('*.html')):run(p,0)
numerics={'1e2':1,'65535.9':65535,'':None,' ':None,'+18.9':18,'-0.5':0,'-1.5':None,'65536.0':None,' 17tail':17,'nan':None,'0x10':0,'999999999999999999999999':None}
for axis in ('width','height'):
    for i,(value,expected) in enumerate(numerics.items()):
        dims=(value,18) if axis=='width' else (18,value)
        p=inputs/f'{axis}_{i}.html';p.write_text(table([[dims]]));r=run(p,0)
        assert bool(r['supported'])==(expected is not None),(axis,value,r)
        if expected is not None:assert r[axis]==expected,(axis,value,r)
(s/'native-results.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps({'runs':len(results),'sanitizer_failures':0,'numeric_cases':len(numerics)*2,'allocation_shapes':list(shapes)}))
