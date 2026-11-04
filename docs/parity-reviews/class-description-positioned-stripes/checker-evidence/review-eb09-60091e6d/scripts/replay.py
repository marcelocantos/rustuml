# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,hashlib,json,os,subprocess,sys,time
from pathlib import Path
from collections import Counter
S=Path(__file__).resolve().parent;R=Path.cwd();P=R/'docs/parity-reviews/class-description-positioned-stripes/maker-evidence/packet6'
ENV=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
variant,scope=sys.argv[1:];binary=S/('target-'+variant)/'release/rustuml';cmp=S/'check_svg'
if scope=='preserved':
 paths=[R/p for p in json.loads((P/'all1649.json').read_text())];paths+=sorted((P/'fresh-pairs').glob('*.puml'))
 assert len(paths)==1675
elif scope=='fresh':paths=sorted((S/'fresh').glob('*.puml'))
elif scope=='nonclass':paths=[R/p for p in json.loads((S/'nonclass.json').read_text())]
else:raise ValueError(scope)
dest=S/(scope+'-'+variant);dest.mkdir(exist_ok=True)
def run(cmd):
 start=time.time()
 try:
  p=subprocess.run([str(x) for x in cmd],env=ENV,capture_output=True,timeout=60)
  return p.returncode,p.stdout,p.stderr,time.time()-start
 except subprocess.TimeoutExpired as e:return 124,e.stdout or b'',(e.stderr or b'')+b'\nCHECKER_TIMEOUT',time.time()-start

def one(it):
 index,src=it;ref=src.with_suffix('.svg');out=dest/f'{index:04d}-{src.stem}';out.mkdir(exist_ok=True)
 records={}
 for mode,args,filename in [('cli',['-tsvg'],'rust.svg'),('ast',['--ast'],'ast.txt'),('yaml',['--yaml'],'model.yaml')]:
  cmd=[binary,*args,src];code,stdout,stderr,elapsed=run(cmd);(out/filename).write_bytes(stdout);(out/(filename+'.stderr')).write_bytes(stderr)
  records[mode]=dict(command=list(map(str,cmd)),exit=code,seconds=elapsed,stdout_sha256=hashlib.sha256(stdout).hexdigest(),stderr_sha256=hashlib.sha256(stderr).hexdigest(),stderr=stderr.decode(errors='replace'))
 cmd=[cmp,ref,out/'rust.svg'];code,stdout,stderr,elapsed=run(cmd);(out/'diff.txt').write_bytes(stdout);(out/'diff.stderr').write_bytes(stderr)
 records['comparison']=dict(command=list(map(str,cmd)),exit=code,seconds=elapsed,stderr=stderr.decode(errors='replace'))
 status='invalid_reference' if code==2 else 'process_error' if records['cli']['exit'] else 'pass' if code==0 else 'mismatch' if code==1 else 'comparison_error'
 return dict(index=index,source=str(src.relative_to(R)) if src.is_relative_to(R) else str(src),source_sha256=sha(src),java_sha256=sha(ref),status=status,outputs=str(out),**records)
start=time.time()
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(one,enumerate(paths)))
report=dict(binary=str(binary),binary_sha256=sha(binary),revision='b074d2082cb9ed2b9460f79a32b170e31cc1ab5e' if variant=='before' else 'eb09cecbc90f5690c4a09bf49118f2feff8c1c12',comparator_sha256=sha(cmp),environment={k:ENV[k] for k in ['PLANTUML_STDLIB','RUSTUML_DEBUG']},counts=dict(Counter(r['status'] for r in rows)),seconds=time.time()-start,results=rows)
(S/(scope+'-'+variant+'.json')).write_text(json.dumps(report,indent=2)+'\n');print(scope,variant,report['counts'],flush=True)
