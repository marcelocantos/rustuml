# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Live CLI checks with host zone rules rather than the fixed gate override."""
import datetime
import importlib.util
import json
from pathlib import Path

ROOT=Path.cwd()
path=ROOT/'docs/parity-reviews/tim-evaluated-event-stream/checker-support/check.py'
spec=importlib.util.spec_from_file_location('check',path)
m=importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
manifest=json.loads((m.SUPPORT/'case-manifest.json').read_text())
fresh=json.loads((m.SUPPORT/'fresh-results.json').read_text())
cases=[]
for name,utc in [('fall_before','2026-04-04T15:59:59+00:00'),('fall_after','2026-04-04T16:00:00+00:00'),('spring_before','2026-10-03T15:59:59+00:00'),('spring_after','2026-10-03T16:00:00+00:00')]:
    stamp=int(datetime.datetime.fromisoformat(utc).timestamp())
    source=Path('test-diagrams/perturbations/tim-explicit-date-instant')/('checker_dst_'+name+'.puml')
    source.write_text('@startuml\n!$stamp = %date("yyyy-MM-dd HH:mm:ss", '+str(stamp)+')\nclass "$stamp" as Transition\n@enduml\n')
    meta={'axes':['instant straddles DST transition','host timezone versus fixed debug offset'],'kind':'fresh checker timezone control'}
    manifest[str(source)]=meta
    fresh.append(m.measure(source,True,meta))
    cases.append(source)
results=[]
for zone in ['Australia/Melbourne','Asia/Kathmandu','UTC']:
    env=dict(m.ENV,TZ=zone,RUSTUML_DEBUG='date=1774210426000')
    for source in cases:
        out=Path(__file__).resolve().parent/'timezones'/zone.replace('/','_')
        out.mkdir(parents=True,exist_ok=True)
        raw=source.read_bytes()
        j=m.run([m.JAVA,'-Djava.awt.headless=true','-Duser.timezone='+zone,'-jar',m.JAR,'-pipe','-tsvg'],raw,env)
        golden=out/(source.stem+'.svg');golden.write_bytes(j.stdout)
        (out/(source.stem+'.puml')).write_bytes(raw)
        r=m.run([m.BIN,'-pipe'],raw,env)
        actual=out/(source.stem+'.rust.svg');actual.write_bytes(r.stdout)
        comparison=m.run([m.SCRATCH/'probe','compare',golden,actual])
        (out/(source.stem+'.comparison.txt')).write_bytes(comparison.stdout+comparison.stderr)
        results.append({'source':str(source),'timezone':zone,'RUSTUML_DEBUG':env['RUSTUML_DEBUG'],'java_exit':j.returncode,'cli_exit':r.returncode,'comparison_exit':comparison.returncode,'result':'pass' if j.returncode==r.returncode==comparison.returncode==0 else 'fail'})
(m.SUPPORT/'case-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
(m.SUPPORT/'fresh-results.json').write_text(json.dumps(fresh,indent=2)+'\n')
(Path(__file__).resolve().parent/'timezone-results.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps(results,indent=2))
