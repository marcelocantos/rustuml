# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent raw CLI measurements. All Java runs through Monkey Man's pinned wrapper."""
import concurrent.futures, hashlib, json, os, shlex, subprocess, sys
from pathlib import Path
import xml.etree.ElementTree as ET
import yaml
ROOT=Path(__file__).resolve().parents[4]
SUP=Path(__file__).resolve().parent
S=Path(os.environ.get('TIM_CHECK_SCRATCH','/tmp/rustuml-tim-checker3.dRQvKH'))
REMOTE=os.environ.get('TIM_CHECK_REMOTE','/Users/marcelo/rustuml-reference/scratch/worker.l4Yx4xIn')
HOST='marcelo@192.168.1.112'; WRAPPER='/Users/marcelo/rustuml-reference/bin/reference'
ENV=dict(os.environ,CARGO_TARGET_DIR=str(S/'target'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
def dump(p,d):
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(d,indent=2)+'\n')
def save(p,b):
 p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
def run(args,data=None,cwd=ROOT,env=ENV):
 try:return subprocess.run([str(a) for a in args],input=data,capture_output=True,cwd=cwd,env=env,timeout=90)
 except subprocess.TimeoutExpired as e:return subprocess.CompletedProcess(args,124,e.stdout or b'',(e.stderr or b'')+b'\nCHECKER_TIMEOUT')
def remote(args,data=None):return run(['ssh','-T','-o','BatchMode=yes',HOST,shlex.join([WRAPPER,*args])],data)
def reference(row):
 p=ROOT/row['source'];out=S/'reference'/p.parent.name/p.stem;out.mkdir(parents=True,exist_ok=True)
 cwd=REMOTE+'/'+row.get('cwd','.')
 args=['--cwd',cwd,'--timezone',row.get('zone','GMT+11:00')]
 r=remote(args+['--','-pipe','-tsvg'],p.read_bytes());save(out/'java.svg',r.stdout);save(out/'java.stderr',r.stderr)
 if r.returncode==255:raise RuntimeError(r.stderr.decode())
 save(p.with_suffix('.svg'),r.stdout)
 q=remote(args+['--probe',REMOTE+'/SourceProbe.java','--classpath',REMOTE,'--',REMOTE+'/'+row['source'],cwd]);save(out/'java.source',q.stdout);save(out/'source.stderr',q.stderr)
 result=dict(row,java_exit=r.returncode,source_exit=q.returncode,java_command=[WRAPPER,*args,'--','-pipe','-tsvg'],source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),java_svg_sha256=hashlib.sha256(r.stdout).hexdigest())
 dump(out/'result.json',result);return result

def measure(row,phase,diagnostics=False):
 p=ROOT/row['source'];out=S/'outputs'/phase/p.parent.name/p.stem;out.mkdir(parents=True,exist_ok=True)
 env=dict(ENV,RUSTUML_DEBUG=row.get('debug',ENV['RUSTUML_DEBUG']))
 if row.get('zone'):env['TZ']=row['zone']
 cwd=ROOT/row.get('cwd','.')
 r=run([S/(phase+'-rustuml'),'-pipe'],p.read_bytes(),cwd,env);save(out/'rust.svg',r.stdout);save(out/'rust.stderr',r.stderr)
 c=run([S/'probe','compare',p.with_suffix('.svg'),out/'rust.svg']);save(out/'comparison.txt',c.stdout+c.stderr)
 try:ET.fromstring(p.with_suffix('.svg').read_bytes());malformed=False
 except ET.ParseError:malformed=True
 result=dict(row,cli_exit=r.returncode,stdout_bytes=len(r.stdout),comparison_exit=c.returncode,svg_match=r.returncode==0 and c.returncode==0,reference_invalid=c.returncode==2 or malformed,panic=b'panicked at' in r.stderr)
 if diagnostics:
  y=run([S/(phase+'-rustuml'),'--yaml','-pipe'],p.read_bytes(),cwd,env);save(out/'rust.yaml',y.stdout);save(out/'yaml.stderr',y.stderr)
  try:
   node=yaml.compose(y.stdout)
   for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
   raw=node.value.encode();save(out/'rust.source',raw)
   j=S/'reference'/p.parent.name/p.stem/'java.source'
   if j.exists():result['source_equal']=raw==j.read_bytes()
  except (ValueError,TypeError,KeyError,AttributeError,yaml.YAMLError):result['source_equal']=False
 return result

def summary(rows):return dict(total=len(rows),matches=sum(r.get('svg_match',False) for r in rows),invalid=sum(r.get('reference_invalid',False) for r in rows),cli_errors=sum(r.get('cli_exit',0)!=0 for r in rows),valid_cli_errors=sum(r.get('cli_exit',0)!=0 and not r.get('reference_invalid',False) for r in rows),panics=sum(r.get('panic',False) for r in rows),sources=sum(r.get('source_equal',False) for r in rows))
if __name__=='__main__':
 phase,suite=sys.argv[1:]
 rows=json.loads((SUP/('frozen-inventory.json' if suite=='frozen' else suite+'-manifest.json')).read_text())
 with concurrent.futures.ThreadPoolExecutor(max_workers=2 if phase=='reference' else 4) as pool:
  results=list(pool.map(reference if phase=='reference' else lambda r:measure(r,phase,suite!='frozen'),rows))
 dump(SUP/(phase+'-'+suite+'.json'),results)
 print(phase,suite,summary(results),flush=True)
 if suite=='frozen' and phase!='reference':
  for c in ['cohort955','cohort1044']: print(c,summary([r for r in results if r[c]]),flush=True)
