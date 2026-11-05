# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json
from pathlib import Path
S=Path(__file__).parent;D=S/'supplement';D.mkdir(exist_ok=True)
cases={
'sub_suffix_valid':('sub-marker-layers',False,['local punctuation suffix','two block names and replay'],'''!$n=0
!startsub alpha
!$n=$n+2
!endsub! ignored
!startsub beta
!$n=$n+5
!endsub . trailing
!includesub alpha
!includesub beta
class "$n" as Suffix'''),
'macro_original_prefix':('original-macro-scanning',False,['variable before multiline call','raw caller prefix versus evaluated prefix'],'''!definelong LINES()
first
second
!enddefinelong
!$p="prefix"
note as N
$p LINES()
end note'''),
'keys_nested_call':('named-key-operands',False,['nested call inside named argument','RPN named map lifetime'],'''!function $inner($x="inner")
!return $x
!endfunction
!function $outer($a="first",$b="last")
!return $a+"/"+$b
!endfunction
!$v=$outer($a="blue",$b=$inner())
class "$v" as Nested'''),
'adjacent_hsl_alpha':('call-resolution-arity',False,['three versus four argument dispatch','typed string versus integer'],'''!$a=%hsl_color(0,100,50)
!$b=%hsl_color(0,100,50,50)
!$c=%hsl_color("120",100,50)
class "$a/$b/$c" as Color'''),
'json_empty_object':('json-container-validation',False,['empty object versus array','terminal missing dot versus JSON typed string'],'''!$n=0
!foreach $x in {}
!$n=80
!endfor
!foreach $x in []
!$n=90
!endfor
!$d={"v":"007"}
!$a=$d.missing
!$b=$d.v+1
class "$n/$a/$b" as Empty''')}
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();rows=[]
for n,(m,neg,axes,body) in cases.items():
 p=D/(n+'.puml');assert not p.exists();p.write_text('@startuml\n'+body+'\n@enduml\n');rows.append(dict(source=str(p),mechanism='tim-'+m,negative=neg,axes=axes,cwd='supplement',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env={}))
manifest=S/'supplement-manifest.json';manifest.write_text(json.dumps(rows,indent=2)+'\n');probe=Path('docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java').resolve()
resources=[dict(local_path=str(p),remote_relative_path='supplement/'+p.name,sha256=sha(p),bytes=p.stat().st_size) for p in sorted(D.glob('*.puml'))]+[dict(local_path=str(probe),remote_relative_path='SourceProbe.java',sha256=sha(probe),bytes=probe.stat().st_size)]
req=dict(manifest_path=str(manifest),manifest_sha256=sha(manifest),case_count=len(rows),resources=resources,wrapper='/Users/marcelo/rustuml-reference/bin/reference',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',case_contracts=[dict(source=r['source'],source_sha256=sha(Path(r['source'])),cwd='<worker>/supplement',timezone=r['zone'],env_overrides={},svg_argv=['reference','--cwd','<worker>/supplement','--timezone',r['zone'],'--','-pipe','-tsvg'],probe_argv=['reference','--cwd','<worker>/supplement','--timezone',r['zone'],'--probe','<worker>/SourceProbe.java','--classpath','<worker>','--','<worker>/supplement/'+Path(r['source']).name,'<worker>/supplement']) for r in rows],output_contract='Preserve exact raw streams/statuses/hashes/actual argv/cwd/env as TIM24, no normalization.')
p=S/'supplement-request.json';p.write_text(json.dumps(req,indent=2)+'\n');print(sha(p));print(sha(manifest))
