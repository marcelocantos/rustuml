# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Maker CLI replay. Reads preserved references; never launches a JVM or SSH."""
import concurrent.futures,gzip,hashlib,importlib.util,json,os,subprocess,sys
from pathlib import Path
import yaml
R=Path.cwd();S=Path('/tmp/rustuml-tim5');C=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3');P=R/'docs/parity-reviews/tim-expression-value-snapshot/checker4-support'
spec=importlib.util.spec_from_file_location('prior',R/'docs/parity-reviews/tim-typed-values-and-binding/checker3-support/check.py');c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
c.ROOT=R;c.S=S
c.ENV.update(CARGO_TARGET_DIR='/tmp/rustuml-tim5-target',JAVA_TOOL_OPTIONS=(os.environ.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip())
phase,suite=sys.argv[1:]
def dump(path,data):path.write_text(json.dumps(data,indent=2)+'\n')
def fresh(row):
 p=R/row['source'];assert hashlib.sha256(p.read_bytes()).hexdigest()==row['source_sha256']
 d=S/'fresh-outputs'/phase/row['id'];d.mkdir(parents=True,exist_ok=True)
 env=dict(c.ENV,TZ=row['zone'],RUSTUML_DEBUG=row['debug'],**row['env']);commands=[]
 for mode,args in [('svg',['-pipe']),('yaml',['--yaml','-pipe'])]:
  cmd=[S/(phase+'-rustuml'),*args];q=c.run(cmd,p.read_bytes(),R/row['cwd'],env)
  (d/('rust.'+mode)).write_bytes(q.stdout);(d/(mode+'.stderr')).write_bytes(q.stderr)
  commands.append(dict(argv=list(map(str,cmd)),cwd=str(R/row['cwd']),env_overrides=dict(row['env'],TZ=row['zone'],RUSTUML_DEBUG=row['debug']),exit=q.returncode))
 cmd=[S/'probe','compare',p.with_suffix('.svg'),d/'rust.svg'];q=c.run(cmd);(d/'comparison.txt').write_bytes(q.stdout+q.stderr)
 old=previous[row['source']]
 result=dict(row,java_exit=old['java_exit'],java_probe_exit=old['java_probe_exit'],cli_exit=commands[0]['exit'],comparison_exit=q.returncode,reference_valid=old['reference_valid'],svg_match=old['reference_valid'] and commands[0]['exit']==0 and q.returncode==0,panic=b'panicked at' in (d/'svg.stderr').read_bytes(),commands=commands,comparison_command=list(map(str,cmd)))
 try:
  node=yaml.compose((d/'rust.yaml').read_bytes())
  for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
  raw=node.value.encode();(d/'rust.source').write_bytes(raw)
  ref=C/('fresh-reference' if row['id'] in fresh_ids else 'supplement-reference')/row['id']/'source.stdout'
  result['source_equal']=old['reference_valid'] and raw==ref.read_bytes()
 except (ValueError,TypeError,KeyError,AttributeError,yaml.YAMLError):result['source_equal']=False
 dump(d/'result.json',result);return result
if suite=='fresh31':
 rows=json.loads((P/'fresh-manifest.json').read_bytes());fresh_ids={r['id'] for r in rows};rows+=json.loads((P/'supplement-manifest.json').read_bytes())
 previous={r['source']:r for name in ['fresh-candidate','supplement-candidate'] for r in json.loads((P/(name+'.json')).read_bytes())}
 measure=fresh
else:
 rows=json.loads(gzip.decompress((P/'collateral-manifest.json.gz').read_bytes()))
 previous={r['source']:r for r in json.loads(gzip.decompress((P/'collateral-results.json.gz').read_bytes()))}
 measure=lambda row:c.measure(row,phase,row['cohort']!='frozen')
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(measure,rows))
dump(S/(phase+'-'+suite+'.json'),results)
summary=dict(total=len(results),matches=sum(r['svg_match'] for r in results),invalid=sum(not r['reference_valid'] if 'reference_valid' in r else r['reference_invalid'] for r in results),panics=sum(r['panic'] for r in results),gains=[r['source'] for r in results if r['svg_match'] and not previous[r['source']]['svg_match']],losses=[r['source'] for r in results if not r['svg_match'] and previous[r['source']]['svg_match']],new_valid_errors=[r['source'] for r in results if r['cli_exit'] and not previous[r['source']]['cli_exit'] and (r.get('reference_valid',not r.get('reference_invalid',False)))])
dump(S/(phase+'-'+suite+'-summary.json'),summary);print(json.dumps(summary,indent=2),flush=True)
if suite=='fresh31':
 for r in results:print(r['id'],r['reference_valid'],r['cli_exit'],r['svg_match'],r['source_equal'])
