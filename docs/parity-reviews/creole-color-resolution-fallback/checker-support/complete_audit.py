# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,pathlib,hashlib,subprocess,re,base64,gzip
S=pathlib.Path(__file__).parent;R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ae47bffd');J=pathlib.Path('/Users/marcelo/work/github.com/plantuml/plantuml');sha=lambda b:hashlib.sha256(b).hexdigest();jrev='71806a23780b04a5ccde2f8ceb5121edad5eb711'
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
j=json.loads((S/'java-source-audit.json').read_bytes());existing={r['path'] for r in j['files']}
for rel in ['klimt/font/UFontFactory.java','klimt/font/UFontFace.java','klimt/color/HColorAutomagic.java','klimt/color/HColorSimple.java','klimt/color/HColors.java']:
 p=J/'src/main/java/net/sourceforge/plantuml'/rel;b=p.read_bytes();assert b==subprocess.check_output(['git','show',jrev+':'+str(p.relative_to(J))],cwd=J)
 if str(p) not in existing:j['files'].append(dict(path=str(p),sha256=sha(b),matches_pin=True))
dump('java-source-audit.json',j)
a=json.loads((S/'audit-summary.json').read_bytes());a['imports']=list({x['revision']:x for x in a['imports']}.values());a['java_files']=len(j['files']);dump('audit-summary.json',a)
# The registry exception is established from source sets, not fixture names.
javasrc=(J/'src/main/java/net/sourceforge/plantuml/klimt/color/ColorTrieNode.java').read_text();names={n.lower() for n in re.findall(r'register\("([^\"]+)"',javasrc)}
rustsrc=(R/'crates/rustuml-render/src/text_render.rs').read_text();css=rustsrc[rustsrc.index('fn css_color_hex'):rustsrc.index('fn write_text_element')];rnames=set()
for line in css.splitlines():
 if '=> Some(' in line:rnames.update(re.findall(r'"([^\"]+)"',line.split('=>')[0]))
assert names-rnames=={'business','application','motivation','strategy','technology','physical','implementation'}
assert rnames-names=={'rebeccapurple'}
dump('registry-source-check.json',dict(java_names=len(names),rust_names=len(rnames),java_only=sorted(names-rnames),rust_only=sorted(rnames-names),grammar_safety='Schemes and multiple gradient separators supported by HColorSet are excluded by unchanged command candidate grammar; permissive resolver branches are not exposed to combined font commands.'))
# Revalidate parent mapping and fixed environment rather than taking parent claims as checker findings.
original=json.loads(gzip.decompress((S/'input-bundle.json.gz').read_bytes()));relay=json.loads(gzip.decompress((S/'parent-relay-input.json.gz').read_bytes()));env=json.loads((S/'reference-results.json').read_bytes())['environment']
assert base64.b64decode(relay['request_base64'],validate=True)==(S/'request.json').read_bytes();assert base64.b64decode(relay['manifest_base64'],validate=True)==(S/'manifest.json').read_bytes()
assert relay['request']==original['request'] and relay['manifest']==original['manifest']
assert len(relay['resources'])==len(original['files'])==32
for a,b in zip(relay['resources'],original['files']):
 assert a['local_path']==b['local_path'] and a['remote_relative_path']==b['remote_relative_path'];assert base64.b64decode(a['data'],validate=True)==base64.b64decode(b['base64'],validate=True)
assert env['wrapper']=='/Users/marcelo/rustuml-reference/bin/reference';assert env['wrapper_sha256']=='6115a3503877999796cb98b9efb6872d9bb8090c2f7b986706919e475dee069e';assert env['locale']=='en_AU' and env['encoding']=='UTF-8'
assert env['headless'] and env['java_revision']==jrev and env['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
assert sha((S/'reference-transport.json').read_bytes())=='60e895baec8c1e2a46e65be426114e4d84ddcadf764b1999d22721aafb0e570c'
transport=json.loads((S/'reference-transport.json').read_bytes());assert base64.b64decode(transport['data'],validate=True)==(S/'reference-results.json.gz').read_bytes()
dump('relay-independent-audit.json',dict(resource_count=32,all_exact=True,java_argv_provenance='derived, not process-traced',historical_fonts_and_graphviz_equivalence='unproved',hashes={n:sha((S/n).read_bytes()) for n in ['request.json','manifest.json','input-bundle.json.gz','parent-relay-input.json.gz','reference-results.json.gz','reference-transport.json','parent-upload-validation.json','parent-relay-validation.json']}))
# No fresh source is an old golden/perturbation, including maker16 and both prior checkers.
known={sha(p.read_bytes()) for d in ['test-diagrams/perturbations','test-diagrams/golden'] for p in (R/d).rglob('*.puml')}
fresh=json.loads((S/'manifest.json').read_bytes());assert all(sha(pathlib.Path(x['source']).read_bytes()) not in known for x in fresh)
dump('fresh-independence.json',dict(fresh=32,distinct=len({sha(pathlib.Path(x['source']).read_bytes()) for x in fresh}),no_existing_source_hashes=True,comparison_source_count=len(known),design='Different consumers, multi-image adjacency, paired explicit base styles, first-close cross-nesting, numeric width selection, valid-special colors, gradient separator and grammar contrasts; source-guided, never adapted to candidate output.'))
# Private snapshot builds avoid git-backed tool outputs; shipped files equal pinned tree.
for phase,revision in [('before','bdfeba117c7af6a527fc66b0acda6d8273fa6d74'),('after','b8905e296b8865e53f3c573abaebf4adb632d751')]:
 rows=[]
 for p in sorted((S/('src-'+phase)/'crates').rglob('*')):
  if p.is_file():
   rel=str(p.relative_to(S/('src-'+phase)));b=p.read_bytes();assert b==subprocess.check_output(['git','show',revision+':'+rel],cwd=R);rows.append(dict(path=rel,sha256=sha(b)))
 dump('source-'+phase+'-hashes.json',rows)
b=json.loads((S/'builds.json').read_bytes())
for row in b:row['binary_sha256']=sha((S/(row['phase']+'-rustuml')).read_bytes())
dump('builds.json',b)
dump('setup-record.json',dict(initial_revision='8ba2e43a3d38ecf27575819a3d25e041d8581cfb',initial_status='clean',branch='review/creole-fallback-independent-ae47',evidence_tip='b8905e296b8865e53f3c573abaebf4adb632d751',snapshot_method='git archive; no extra worktrees or children',makefile='Unchanged tracked Makefile included in both snapshots via git archive; no extra setup or generated Makefile was necessary. Cargo release builds and oracle tests succeeded in reaching execution.',submodule_attempts=[dict(argv=['git','submodule','update','--init','test-diagrams/golden'],exit=1,stderr="fatal: transport 'file' not allowed; local clone retried and aborted"),dict(argv=['git','-c','protocol.file.allow=always','submodule','update','--init','test-diagrams/golden'],exit=0,revision='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a')],golden_snapshot='Snapshots symlink required golden checkout read-only; diagnostic files are emitted outside the submodule in private snapshot test-diagrams.',broad_gates='Executed once on stable evidence-tip snapshot with 1912 standing pairs before registering checker32; fresh32 compared through independently built unchanged comparator. No broad rerun.'))
print('complete audit verified',len(j['files']),'Java sources')
