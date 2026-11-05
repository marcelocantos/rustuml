# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Maker measurements: local CLI/comparator; all Java exclusively remote."""
import concurrent.futures, hashlib,json,os,shlex,subprocess,sys
from pathlib import Path
import yaml
import xml.etree.ElementTree as ET
S=Path(os.environ.get('TIM_MAKER_SCRATCH','/tmp/rustuml-tim3')); ROOT=Path.cwd();SUP=ROOT/'docs/parity-reviews/tim-expression-event-sink/checker2-support'
REMOTE='/Users/marcelo/rustuml-reference/scratch/worker.Bab5N6zr';WRAPPER='/Users/marcelo/rustuml-reference/bin/reference'
ENV=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
ENV['JAVA_TOOL_OPTIONS']=(ENV.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip()
def run(args,data=None,env=ENV,cwd=ROOT):
 try: return subprocess.run(list(map(str,args)),input=data,capture_output=True,env=env,cwd=cwd,timeout=90)
 except subprocess.TimeoutExpired as e:return subprocess.CompletedProcess(args,124,e.stdout or b'',(e.stderr or b'')+b'\nMAKER TIMEOUT')
def save(path,data):path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
def dump(path,data):save(path,(json.dumps(data,indent=2)+'\n').encode())
def remote(args,raw=None):
 # Only SSH may execute reference Java. Wrapper forces headless and exact JAR.
 return run(['ssh','-T','-o','BatchMode=yes','marcelo@192.168.1.112',shlex.join([WRAPPER,*args])],raw)
def reference(row):
 p=Path(row['source']);out=S/'reference'/p.stem;out.mkdir(parents=True,exist_ok=True)
 if (out/'result.json').exists():return json.loads((out/'result.json').read_text())
 args=['--cwd',REMOTE+'/new','--timezone',row.get('zone','GMT+11:00')]
 r=remote(args+['--','-pipe','-tsvg'],p.read_bytes());save(out/'java.svg',r.stdout);save(out/'java.stderr',r.stderr)
 j=remote(args+['--probe',REMOTE+'/SourceProbe.java','--classpath',REMOTE,'--',REMOTE+'/new/'+p.name,REMOTE+'/new']);save(out/'java.source',j.stdout);save(out/'source.stderr',j.stderr)
 result=dict(row,java_exit=r.returncode,source_exit=j.returncode,argv=[WRAPPER,*args,'--','-pipe','-tsvg'],source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),runtime='Monkey Man OpenJDK21.0.5 Graphviz12.2.0 en_AU; exact campaign JAR; headless forced',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',java_tool_options=ENV['JAVA_TOOL_OPTIONS'])
 dump(out/'result.json',result);return result
def measure(row,phase,suite):
 p=Path(row['source']);out=S/'outputs'/phase/suite/p.parent.name/p.stem;out.mkdir(parents=True,exist_ok=True)
 env=dict(ENV,RUSTUML_DEBUG=row.get('debug',ENV['RUSTUML_DEBUG']))
 if row.get('zone'):env['TZ']=row['zone']
 cwd=S/'new' if suite=='new' else ROOT
 reference=S/'reference'/p.stem/'java.svg' if suite=='new' else p.with_suffix('.svg')
 r=run([S/(phase+'-rustuml'),'-pipe'],p.read_bytes(),env,cwd);save(out/'rust.svg',r.stdout);save(out/'rust.stderr',r.stderr)
 c=run([os.environ.get('TIM_COMPARATOR','/tmp/rustuml-tim-correction/probe'),'compare',reference,out/'rust.svg']);save(out/'comparison.txt',c.stdout+c.stderr)
 try:ET.fromstring(reference.read_bytes());malformed=False
 except ET.ParseError:malformed=True
 result=dict(row,cli_exit=r.returncode,stdout_bytes=len(r.stdout),comparison_exit=c.returncode,svg_match=r.returncode==0 and c.returncode==0,reference_invalid=c.returncode==2 or malformed)
 if suite in ('new','fresh','focused'):
  y=run([S/(phase+'-rustuml'),'--yaml','-pipe'],p.read_bytes(),env,cwd);save(out/'rust.yaml',y.stdout);save(out/'yaml.stderr',y.stderr)
  try:
   node=yaml.compose(y.stdout)
   for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
   save(out/'rust.source',node.value.encode())
   j=S/'reference'/p.stem/'java.source' if suite=='new' else S/'checker-archive'/'outputs'/'candidate'/p.parent.name/p.stem/'java.source'
   if j.exists():result['source_equal']=j.read_bytes()==node.value.encode()
  except Exception:result['source_equal']=False
 return result
if __name__=='__main__':
 phase,suite=sys.argv[1:]
 if suite=='new':
  rows=json.loads((S/'new-manifest.json').read_text())
  rows=[dict(row,source=str(S/'new'/Path(row['source']).name)) for row in rows]
 elif suite=='fresh':rows=json.loads((SUP/'final-manifest.json').read_text())
 elif suite=='isolated':rows=[r for r in json.loads((SUP/'parent-inventory.json').read_text()) if Path(r['source']).exists()]
 elif suite=='focused':rows=[r for r in json.loads((SUP/'final-manifest.json').read_text()) if r['kind'].startswith('fresh')]
 else:raise ValueError(suite)
 with concurrent.futures.ThreadPoolExecutor(max_workers=2 if phase=='reference' else 4) as pool:
  results=list(pool.map(reference if phase=='reference' else lambda r:measure(r,phase,suite),rows))
 dump(S/(phase+'-'+suite+'.json'),results)
 print(json.dumps(dict(phase=phase,suite=suite,total=len(results),svg_match=sum(r.get('svg_match',False) for r in results),invalid_reference=sum(r.get('reference_invalid',False) for r in results),cli_error=sum(r.get('cli_exit',0)!=0 for r in results),source_equal=sum(r.get('source_equal',False) for r in results))))
