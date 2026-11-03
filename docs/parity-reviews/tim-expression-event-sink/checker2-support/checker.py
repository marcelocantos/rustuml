# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent checker: unchanged strict comparator, actual CLI, raw Java pipe."""
import concurrent.futures, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path
import yaml
import xml.etree.ElementTree as ET
ROOT=Path.cwd(); S=Path('/tmp/rustuml-tim-check-2fa74908')
SUP=ROOT/'docs/parity-reviews/tim-expression-event-sink/checker2-support'
JAVA='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
JAR='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
ENV=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
def run(args,data=None,env=ENV):
 try:return subprocess.run(list(map(str,args)),input=data,capture_output=True,env=env,timeout=60)
 except subprocess.TimeoutExpired as e:return subprocess.CompletedProcess(args,124,e.stdout or b'',(e.stderr or b'')+b'\nCHECKER TIMEOUT')
def save(p,b):p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
def dump(p,obj):save(p,(json.dumps(obj,indent=2)+'\n').encode())
def init():
 SUP.mkdir(parents=True,exist_ok=True)
 paths=run(['git','ls-tree','-r','--name-only','29642ec630e9902c892162a7081cd8a5e1113dac','test-diagrams/perturbations']).stdout.decode().splitlines()
 inv=[{'source':p,'source_sha256':hashlib.sha256(Path(p).read_bytes()).hexdigest(),'golden_sha256':hashlib.sha256(Path(p).with_suffix('.svg').read_bytes()).hexdigest()} for p in paths if p.endswith('.puml')];assert len(inv)==955
 dump(SUP/'parent-inventory.json',inv)
 rows=json.loads((S/'manifest.json').read_text())
 for r in rows:
  p=Path('test-diagrams/perturbations')/r['mechanism']/Path(r['source']).name;save(p,Path(r['source']).read_bytes());r['source']=str(p);r['kind']='fresh'
 inc=Path('test-diagrams/perturbations/tim-parser-error-propagation/checker2_error_body.inc');save(inc,b'!$bad = %date("QQ", 0)\n')
 for m in ['tim-evaluated-event-stream','tim-procedure-global-fork-memory','tim-literal-variable-lookup','tim-explicit-date-instant','tim-expression-event-sink','tim-declaration-default-values','tim-expression-quoted-tokens','tim-date-pattern-tokens','tim-parser-error-propagation']:
  for p in sorted((Path('docs/parity-reviews')/m/'maker-cases').glob('*.puml')):
   target=Path('test-diagrams/perturbations')/m/('checker2_retained_'+p.name);save(target,p.read_bytes())
   rows.append(dict(source=str(target),mechanism=m,kind='retained maker',original_source=str(p),axes=['retained maker input','fresh raw-pipe Java reference'],negative='invalid' in p.name or p.stem in ['guard_changed_theme','outside_i32']))
 dump(SUP/'manifest.json',rows)
 print('prepared',len(rows),flush=True)
def measure(item,phase,fresh=False):
 row=dict(item);p=Path(row['source']);raw=p.read_bytes();out=S/'outputs'/phase/p.parent.name/p.stem;out.mkdir(parents=True,exist_ok=True)
 env=dict(ENV,RUSTUML_DEBUG=row.get('debug',ENV['RUSTUML_DEBUG']))
 if row.get('zone'):env['TZ']=row['zone']
 if fresh:
  j=run([JAVA,'-Djava.awt.headless=true','-Duser.timezone='+row.get('zone','GMT+11:00'),'-jar',JAR,'-pipe','-tsvg'],raw,env)
  save(p.with_suffix('.svg'),j.stdout);save(out/'java.stderr',j.stderr);row['java_exit']=j.returncode
  jsrc=run([JAVA,'-Djava.awt.headless=true','-Duser.timezone='+row.get('zone','GMT+11:00'),'-cp',str(S)+':'+JAR,'ReaderProbe','source',p],env=env)
  save(out/'java.source',jsrc.stdout);save(out/'java-source.stderr',jsrc.stderr);row['java_source_exit']=jsrc.returncode
 r=run([S/(phase+'-rustuml'),'-pipe'],raw,env);save(out/'rust.svg',r.stdout);save(out/'rust.stderr',r.stderr)
 c=run([S/'probe','compare',p.with_suffix('.svg'),out/'rust.svg']);save(out/'comparison.txt',c.stdout+c.stderr)
 row.update(cli_exit=r.returncode,stdout_bytes=len(r.stdout),comparison_exit=c.returncode,svg_match=r.returncode==0 and c.returncode==0)
 try:
  ET.fromstring(p.with_suffix('.svg').read_bytes()); malformed=False
 except ET.ParseError:malformed=True
 row['reference_invalid']=c.returncode==2 or malformed
 if fresh:
  row['java_valid']=row['java_exit']==0 and not row['reference_invalid']
  row['negative_rejection']=not row['java_valid'] and r.returncode!=0 and not r.stdout
  y=run([S/(phase+'-rustuml'),'--yaml','-pipe'],raw,env);save(out/'rust.yaml',y.stdout);save(out/'rust-yaml.stderr',y.stderr);row['metadata_exit']=y.returncode
  try:
   node=yaml.compose(y.stdout)
   for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
   save(out/'rust.source',node.value.encode());row['source_equal']=node.value.encode()==jsrc.stdout
  except Exception as e:row['source_equal']=False;row['metadata_error']=str(e)
 return row
def main():
 phase,suite=sys.argv[1:];fresh=suite=='fresh'
 rows=json.loads((SUP/('final-manifest.json' if fresh else 'parent-inventory.json')).read_text())
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(lambda r:measure(r,phase,fresh),rows))
 dump(SUP/(phase+'-'+suite+'.json'),results)
 print(json.dumps(dict(phase=phase,suite=suite,total=len(results),svg_match=sum(r['svg_match'] for r in results),invalid_reference=sum(r['reference_invalid'] for r in results),cli_error=sum(r['cli_exit']!=0 for r in results),source_equal=sum(r.get('source_equal',False) for r in results))))
if __name__=='__main__':
 if sys.argv[1]=='init':init()
 else:main()
