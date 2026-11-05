# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,gzip,hashlib,json,os,subprocess,sys,xml.etree.ElementTree as ET,collections
from pathlib import Path
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-78c595f4');C=Path(__file__).parent
E=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
def sha(b):return hashlib.sha256(b).hexdigest()
def run(argv,data=None):
 try:
  p=subprocess.run([str(x) for x in argv],cwd=R,env=E,input=data,capture_output=True,timeout=90)
  return dict(argv=[str(x) for x in argv],exit=p.returncode,stdout=p.stdout.decode(errors='replace'),stderr=p.stderr.decode(errors='replace'))
 except subprocess.TimeoutExpired as e:return dict(argv=[str(x) for x in argv],exit=124,stdout=(e.stdout or b'').decode(errors='replace'),stderr='TIMEOUT '+(e.stderr or b'').decode(errors='replace'))
def inventory(s):
 try:
  root=ET.fromstring(s);tags=collections.Counter();labels=collections.Counter();heads=[];polygons=[];texts=[]
  for e in root.iter():
   tag=e.tag.split('}')[-1];tags[tag]+=1
   if tag=='text':
    label=''.join(e.itertext());labels[label]+=1;texts.append(dict(label=label,attributes=e.attrib))
   if tag=='g' and 'participant' in e.get('class','').split():heads.append(e.attrib)
   if tag=='polygon':polygons.append(e.attrib)
  return dict(tags=dict(tags),labels=dict(labels),heads=heads,polygons=polygons,texts=texts,canvas=root.attrib)
 except ET.ParseError as e:return dict(xml_error=str(e))
def main(manifest,prefix):
 items=json.loads(Path(manifest).read_text());out=C/prefix;out.mkdir(exist_ok=False)
 def one(pair):
  index,item=pair;src=R/item['source'];gold=R/item['golden'];body=src.read_bytes();ref=gold.read_bytes();row=dict(item,index=index,source_sha256=sha(body),golden_sha256=sha(ref),reference_inventory=inventory(ref),results={})
  for rev in ['before','after']:
   b=C/(rev+'-bin');v={}
   for mode,flags in [('cli',['-tsvg','-pipe']),('ast',['--ast','-pipe']),('ascii',['-ttxt','-pipe'])]:
    r=run([b/'rustuml',*flags],body);s=r.pop('stdout');path=out/(str(index)+'-'+rev+{'cli':'.svg','ast':'.ast','ascii':'.txt'}[mode]);path.write_text(s);r.update(output=path.name,sha256=sha(path.read_bytes()))
    if mode=='cli':r['inventory']=inventory(s);v['cli_comparison']=run([b/'compare_cli',gold,path])
    v[mode]=r
   for mode,flags in [('product_library',['--no-oracle']),('assisted_library',[])]:v[mode]=run([b/'diff_one',*flags,src])
   row['results'][rev]=v
  return row
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
  rows=[]
  for row in pool.map(one,enumerate(items)):
   rows.append(row)
   if len(rows)%200==0:print(prefix,len(rows),flush=True)
 (out/'measurements.json.gz').write_bytes(gzip.compress(json.dumps(rows,indent=2).encode(),mtime=0))
 print(prefix,'complete',len(rows),flush=True)
if __name__=='__main__':main(sys.argv[1],sys.argv[2])
