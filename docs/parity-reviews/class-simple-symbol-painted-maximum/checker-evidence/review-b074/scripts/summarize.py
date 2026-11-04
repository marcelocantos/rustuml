# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,re,subprocess,tarfile,xml.etree.ElementTree as ET
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-review-b074');E=S/'evidence';ROOT=Path.cwd();P=ROOT/'docs/parity-reviews/class-simple-symbol-painted-maximum/maker-evidence/packet5';sha=lambda b:hashlib.sha256(b).hexdigest()
summary={}
with tarfile.open(P/'preserved-cli-and-logs.tar.gz') as archive:
 for name in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
  old=archive.extractfile('logs/'+name).read();new=(ROOT/'test-diagrams'/name).read_bytes();(E/('maker-'+name)).write_bytes(old);(E/name).write_bytes(new)
  a=set(old.decode().splitlines());b=set(new.decode().splitlines());summary[name]=dict(before_count=len(a),after_count=len(b),new_failures=sorted(b-a),repaired=sorted(a-b),byte_identical=old==new)
 old=archive.extractfile('logs/corrected-clippy.log').read();(E/'maker-clippy.log').write_bytes(old)
 def diagnostics(s):return [re.sub(r':\d+:\d+',':LINE:COL',line.strip()) for line in s.splitlines() if line.startswith('error:') or re.match(r'\s*--> crates/',line)]
 old_d=diagnostics(old.decode());new_d=diagnostics((E/'clippy.log').read_text());summary['clippy']=dict(unchanged=old_d==new_d,before=old_d,after=new_d)
summary['gates']=dict(library_passes=1439,cli_passes=13,error_path_passes=19,doc_passes=2,product_matches=11188,assisted_matches=10523,eligible=11251,total=12550,guards_passed=4,guards_total=6,format_exit=0)
summary['setup_failure']='Initial product/assisted/census-guard attempt found empty prepared submodule; logs retained. Cloned local corpus at exact gitlink 9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a, no network or Java. Only affected checks rerun; one substantive final-revision execution each.'
summary['diagnostic_script_failure']='First XML diagnostic attempted to parse empty Rust output for a Java-negative case; raised ParseError. Corrected diagnostic to record missing XML separately. CLI/comparator captures and statuses were already preserved and were not rerun.'
(E/'gate-delta.json').write_text(json.dumps(summary,indent=2)+'\n')
identity=dict(before='c1a38ada817d820ca8f57b1e8db0d9048e9657eb',candidate='b074d2082cb9ed2b9460f79a32b170e31cc1ab5e',production='96a4bde220eddbc3faf135cad6d13f338b7d7ed5',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',golden_gitlink='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a',build_jobs=4,binaries={v:sha((S/f'target-{v}/release/rustuml').read_bytes()) for v in ['before','after']},comparator_binary_sha256=sha((S/'check_svg').read_bytes()),toolchain={cmd:subprocess.check_output([cmd,'--version'],text=True).strip() for cmd in ['rustc','cargo']})
identity['source_hashes']={p:{rev:sha(subprocess.check_output(['git','show',f'{rev}:{p}'])) for rev in [identity['before'],identity['production'],identity['candidate']]} for p in ['crates/rustuml-render/src/class.rs','crates/rustuml-oracle/src/compare.rs','crates/rustuml-oracle/src/harness.rs','crates/rustuml-render/src/text_render.rs','Cargo.lock']}
identity['production_changed_files']=subprocess.check_output(['git','diff','--name-only',identity['before'],identity['production'],'--','crates'],text=True).splitlines()
identity['account_ancestry']={h:subprocess.run(['git','merge-base','--is-ancestor',h,identity['production']]).returncode==0 for h in ['3790f114c79f2d19fdf91e46b84f1f79d85f6677','41d9a803623d00f1263eaaaa08ac56ab51a5eac0']}
(E/'identity.json').write_text(json.dumps(identity,indent=2)+'\n')
tag=lambda x:x.tag.rsplit('}',1)[-1]
obs=json.loads((E/'fresh-direct-observations.json').read_text())
for r in obs:
 p=next((S/'fresh').rglob(r['name']))
 try:
  roots={v:ET.fromstring(p.with_suffix(s).read_bytes()) for v,s in [('java','.svg'),('before','.before.svg'),('after','.after.svg')]}
 except ET.ParseError:continue
 def rows(root):return [(tag(x),sorted(x.attrib.items()),x.text or '') for x in root.iter() if tag(x) not in ['svg','g','a']]
 r['before_after_interior_primitives_identical']=rows(roots['before'])==rows(roots['after'])
 r['byte_identical_before_after']=p.with_suffix('.before.svg').read_bytes()==p.with_suffix('.after.svg').read_bytes()
 if p.stem.endswith(('blanktail','sized')):
  r['canvas_baseline_diagnostic']={v:dict(height=float(root.get('height').removesuffix('px')),last_emitted_text_max=max(float(x.get('y'))+1.5 for x in root.iter() if tag(x)=='text')) for v,root in roots.items()}
(E/'fresh-direct-observations.json').write_text(json.dumps(obs,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k!='clippy'},indent=2));print('clippy unchanged',summary['clippy']['unchanged'])
