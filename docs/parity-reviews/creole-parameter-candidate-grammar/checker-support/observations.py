# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess,xml.etree.ElementTree as E,re,base64,gzip
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();M=S/'maker';J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');sha=lambda b:hashlib.sha256(b).hexdigest();dump=lambda name,x:(S/name).write_text(json.dumps(x,indent=2)+'\n')
# Pin the additional source mechanisms used by this checker.
extras=['klimt/creole/atom/AtomImg.java','klimt/compress/Slot.java','klimt/compress/SlotFinder.java','klimt/creole/command/CommandCreoleOpenIcon.java']
rows=[]
for rel in extras:
 p=J/'src/main/java/net/sourceforge/plantuml'/rel;b=p.read_bytes();assert b==subprocess.check_output(['git','-C',str(J),'show','71806a23780b04a5ccde2f8ceb5121edad5eb711:'+str(p.relative_to(J))]);rows.append(dict(path=str(p.relative_to(J)),sha256=sha(b),matches_pin=True))
dump('additional-java-check.json',rows)
# Lossless relay envelopes independently verified again, including the raw transport.
transport=json.loads((S/'reference-transport.json').read_bytes());assert base64.b64decode(transport['data'],validate=True)==(S/'reference-results.json.gz').read_bytes()
relay=json.loads(gzip.decompress((S/'parent-relay-input.json.gz').read_bytes()));original=json.loads(gzip.decompress((S/'input-bundle.json.gz').read_bytes()))
assert base64.b64decode(relay['request_base64'],validate=True)==(S/'request.json').read_bytes();assert base64.b64decode(relay['manifest_base64'],validate=True)==(S/'manifest.json').read_bytes();assert relay['request']==original['request'] and relay['manifest']==original['manifest']
assert len(relay['resources'])==len(original['files'])==36
for a,b in zip(relay['resources'],original['files']):assert a['local_path']==b['local_path'] and a['remote_relative_path']==b['remote_relative_path'] and base64.b64decode(a['data'],validate=True)==base64.b64decode(b['base64'],validate=True)
dump('relay-envelope-check.json',dict(lossless=True,files=36,hashes={n:sha((S/n).read_bytes()) for n in ['request.json','manifest.json','input-bundle.json.gz','parent-relay-input.json.gz','reference-results.json.gz','reference-transport.json']}))
# Compare initial/final diagnostic failure sets, and every separately discovered golden control.
reg=[]
phases={p:json.loads((S/('golden-regressions-'+p+'.json')).read_text()) for p in ['before','initial','after']}
allsource={r['source_sha256'] for p in M.glob('*-membership.json') if '-original-' not in p.name and p.name!='golden-regressions-membership.json' for r in json.loads(p.read_text())}
for r in json.loads((S/'golden-regressions-membership.json').read_text()):
 records={p:next(v for v in vs if v['id']==r['id']) for p,vs in phases.items()};gold=R/'test-diagrams/golden'/r['id'].removeprefix('golden-regressions/');assert gold.with_suffix('.puml').read_bytes()==Path(r['source_capture']).read_bytes();assert gold.with_suffix('.svg').read_bytes()==Path(r['golden_capture']).read_bytes()
 reg.append(dict(id=r['id'],source_present_in_1992=r['source_sha256'] in allsource,**{p:v['svg_match'] for p,v in records.items()},models_byte_equal=len({(S/'replay'/p/r['id']/'yaml.stdout').read_bytes() for p in phases})==1))
fn=[]
for n in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 b=(S/('before-'+n)).read_bytes();a=(S/n).read_bytes();i=(S/('initial-'+n)).read_bytes();fn.append(dict(file=n,before_count=len(b.splitlines()),initial_count=len(i.splitlines()),after_count=len(a.splitlines()),after_byte_equal_before=a==b,after_byte_equal_maker_baseline=a==(M/'baseline-diagnostics'/n).read_bytes(),initial_losses=sorted(set(i.decode().splitlines())-set(b.decode().splitlines())),initial_gains=sorted(set(b.decode().splitlines())-set(i.decode().splitlines())),final_sha256=sha(a)))
dump('regression-check.json',dict(cases=reg,failure_names=fn,coverage_finding='All six golden source hashes absent from every replay membership (1992). Initial focused creole-only tests and replay did not catch them; maker initial broad library/unit gates did. New independent initial diagnostics reproduce six product and five assisted losses, final repair restores all.'))
# Capture exact text/style geometry and malformed XML, separately from comparison verdicts.
texts=[];malformed={};leaks=[]
for c in ['checker36','fresh16','checker28']:
 rows=json.loads((S/(c+'-membership.json')).read_text())
 for row in rows:
  n=row['id'];entry=dict(id=n)
  for phase,path in [('java',Path(row['golden_capture'])),('before',S/'replay/before'/n/'svg.stdout'),('after',S/'replay/after'/n/'svg.stdout')]:
   b=path.read_bytes()
   try:
    root=E.fromstring(b);entry[phase]=[dict(text=''.join(t.itertext()),attributes=t.attrib) for t in root.iter() if t.tag.endswith('}text')]
    for t in root.iter():
     for k,v in t.attrib.items():
      if any(0xE000<=ord(ch)<=0xE0FF for ch in v):leaks.append(dict(id=n,phase=phase,attribute=k,value=v))
   except E.ParseError as e:entry[phase]=dict(xml_error=str(e));malformed.setdefault(phase,[]).append(n)
  texts.append(entry)
dump('text-observations.json',texts);dump('hidden-attribute-observations.json',leaks);dump('malformed-svg-observations.json',malformed)
# Exact source snapshot and compile identities (evidence overlays never modify crates).
builds=[]
for phase in ['before','initial','after']:
 D=S/('source-'+phase);rev=(D/'REVISION').read_text().strip();files=[]
 for p in subprocess.check_output(['git','ls-tree','-r','--name-only',rev,'--','crates','vendor','Cargo.toml','Cargo.lock']).decode().splitlines():
  b=(D/p).read_bytes();obj=subprocess.check_output(['git','rev-parse',rev+':'+p]).decode().strip();assert hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()==obj
  files.append(dict(path=p,sha256=sha(b),git_blob=obj))
 builds.append(dict(phase=phase,revision=rev,binary_sha256=sha((S/(phase+'-rustuml')).read_bytes()),sources=files))
dump('build-source-provenance.json',dict(builds=builds,rustc=subprocess.check_output(['rustc','-Vv']).decode(),cargo=subprocess.check_output(['cargo','-V']).decode(),jobs=4,private_targets_initially_empty=True,after_evidence_overlay='docs and test-diagrams from bdfeba117, plus original exact Makefile restored after setup failure; crates/vendor/Cargo checked by blob'))
print('regressions',reg);print('hidden attributes',len(leaks),set(x['attribute'] for x in leaks if x['phase']=='after'));print('malformed',malformed)
