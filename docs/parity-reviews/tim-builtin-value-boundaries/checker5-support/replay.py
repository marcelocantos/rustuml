# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Replay fixed memberships with fresh isolated before/after CLI binaries. No JVM."""
import concurrent.futures,gzip,hashlib,json,os,subprocess,sys,xml.etree.ElementTree as ET
from pathlib import Path
import yaml
S=Path(__file__).parent;R=Path.cwd();P=R/'docs/parity-reviews/tim-expression-value-snapshot/checker4-support';M=R/'docs/parity-reviews/tim-builtin-value-boundaries/maker-support';C=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3')
def load(p):return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
def sha(b):return hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
def model(b):
 root={k.value:v for k,v in yaml.compose(b).value};d={k.value:v for k,v in root['diagram'].value};meta={k.value:v for k,v in d['meta'].value};return dict(type=root['type'].value,entities=len(d['entities'].value) if 'entities' in d else None,source=meta['source'].value)
def run(cmd,cwd,env,data=None):
 try:return subprocess.run(list(map(str,cmd)),input=data,cwd=cwd,env=env,capture_output=True,timeout=90)
 except subprocess.TimeoutExpired as e:return subprocess.CompletedProcess(cmd,124,e.stdout or b'',(e.stderr or b'')+b'\nCHECKER_TIMEOUT')
suite=sys.argv[1]
if suite=='collateral':
 rows=load(P/'collateral-manifest.json.gz')
 for row in rows:
  p=Path(row['source']);ref=C/'baseline-capture/reference'/p.parent.name/p.stem/'java.source'
  if ref.exists():row['reference_source']=str(ref)
elif suite=='prior31':
 rows=load(P/'fresh-manifest.json')+load(P/'supplement-manifest.json')
 for row in rows:
  for sub in ['fresh-reference','supplement-reference']:
   p=S/'checker4-archive'/sub/row['id']/'source.stdout'
   if p.exists():row['reference_source']=str(p)
elif suite in ['maker15','maker17']:
 base=M if suite=='maker15' else R/'docs/parity-reviews/tim-expression-value-snapshot/maker-support'
 rows=load(base/'new-manifest.json')
 for row in rows:
  p=base/'inputs'/Path(row['source']).name;row['source']=str(p);row['cwd']=str(p.parent)
  ref=base/('relay15' if suite=='maker15' else 'relay-completion')/p.stem
  row['golden']=str(ref/('java.svg.stdout' if suite=='maker15' else 'java.svg'))
  row['reference_source']=str(ref/('java.source.stdout' if suite=='maker15' else 'java.source'))
elif suite in ['fresh24','supplement5']:
 rows=load(S/('new-manifest.json' if suite=='fresh24' else 'supplement-manifest.json'))
 for row in rows:
  row['cwd']=str(S/('new' if suite=='fresh24' else 'supplement'));row['golden']=str(S/('relay24' if suite=='fresh24' else 'relay5')/Path(row['source']).stem/'java.svg.stdout');row['reference_source']=str(S/('relay24' if suite=='fresh24' else 'relay5')/Path(row['source']).stem/'java.source.stdout')
else:raise ValueError(suite)
def measure(row):
 p=R/row['source'];gold=Path(row.get('golden',p.with_suffix('.svg')));data=p.read_bytes();gb=gold.read_bytes()
 for field,b in [('source_sha256',data),('golden_sha256',gb)]:
  if field in row:assert sha(b)==row[field],str(p)
 id=sha(str(p).encode())[:14]+'-'+p.stem; result=dict(row,source_sha256=sha(data),golden_sha256=sha(gb),capture_id=id,phases={})
 try:ET.fromstring(gb);xml_valid=True
 except ET.ParseError:xml_valid=False
 result['reference_xml_valid']=xml_valid
 for phase in ['before','after']:
  d=S/'replay'/suite/phase/id;d.mkdir(parents=True,exist_ok=True)
  env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG=row.get('debug','date=1774210426000,tz=AEDT+1100'),TZ=row.get('zone','GMT+11:00'),**row.get('env',{}));cwd=R/row.get('cwd','.')
  binary=S/('target-'+phase)/'release/rustuml';calls=[]
  for mode,argv in [('svg',[binary,'-pipe']),('yaml',[binary,'--yaml','-pipe'])]:
   if mode=='yaml' and suite=='collateral' and row.get('cohort')=='frozen' and 'reference_source' not in row:continue
   q=run(argv,cwd,env,data);(d/('rust.'+mode)).write_bytes(q.stdout);(d/(mode+'.stderr')).write_bytes(q.stderr)
   calls.append(dict(mode=mode,argv=list(map(str,argv)),cwd=str(cwd),env_overrides={k:env[k] for k in ['TZ','RUSTUML_DEBUG','PLANTUML_STDLIB','RAYON_NUM_THREADS']}|row.get('env',{}),exit=q.returncode,stdout_sha256=sha(q.stdout),stderr_sha256=sha(q.stderr)))
  cmd=[S/'probe','compare',gold,d/'rust.svg'];q=run(cmd,R,env);(d/'comparison.stdout').write_bytes(q.stdout);(d/'comparison.stderr').write_bytes(q.stderr)
  valid=xml_valid and q.returncode!=2
  result['reference_valid']=valid
  z=dict(cli_exit=calls[0]['exit'],comparison_exit=q.returncode,svg_match=valid and calls[0]['exit']==0 and q.returncode==0,panic=b'panicked at' in (d/'svg.stderr').read_bytes(),calls=calls,comparison_argv=list(map(str,cmd)))
  if (d/'rust.yaml').exists():
   try:
    z['model']=model((d/'rust.yaml').read_bytes());src=z['model']['source'].encode();(d/'rust.source').write_bytes(src)
    if 'reference_source' in row:
     ref=Path(row['reference_source']).read_bytes();z['source_equal']=valid and src==ref;z['reference_source_sha256']=sha(ref)
   except (TypeError,ValueError,KeyError,AttributeError,yaml.YAMLError):z['source_equal']=False
  dump(d/'result.json',z);result['phases'][phase]=z
 return result
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:out=list(pool.map(measure,rows))
dump(S/(suite+'-results.json'),out)
summary=dict(total=len(out),valid=sum(r['reference_valid'] for r in out),before=sum(r['phases']['before']['svg_match'] for r in out),after=sum(r['phases']['after']['svg_match'] for r in out),source_matches=sum(r['phases']['after'].get('source_equal',False) for r in out),negative_rejected=sum(not r['reference_valid'] and r['phases']['after']['cli_exit']!=0 for r in out),panics=sum(r['phases']['after']['panic'] for r in out),losses=[r['source'] for r in out if r['phases']['before']['svg_match'] and not r['phases']['after']['svg_match']],new_valid_errors=[r['source'] for r in out if r['reference_valid'] and not r['phases']['before']['cli_exit'] and r['phases']['after']['cli_exit']])
for cohort in ['cohort955','cohort1044']:
 summary[cohort]=dict(total=sum(bool(r.get(cohort)) for r in out),after=sum(r['phases']['after']['svg_match'] for r in out if r.get(cohort)))
dump(S/(suite+'-summary.json'),summary);print(suite,json.dumps(summary),flush=True)
if suite in ['fresh24','supplement5']:
 for r in out:print(Path(r['source']).stem,r['reference_valid'],r['phases']['after']['cli_exit'],r['phases']['after']['svg_match'],r['phases']['after'].get('source_equal'),r['phases']['after'].get('model',{}).get('source',''))
