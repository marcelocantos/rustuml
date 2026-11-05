# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package immutable captures and independently authored review evidence only."""
import collections,gzip,hashlib,json,pathlib,shutil,subprocess,tarfile
C=pathlib.Path(__file__).parent
R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ef020aa1')
D=R/'docs/parity-reviews/sequence-corrective-b9/checker-ef020'
D.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
def read(p):return json.loads(p.read_text())
def put(p,x):p.write_text(json.dumps(x,indent=2)+'\n')
findings=read(C/'findings.json')
findings.extend([
 dict(id='raw-link-second-display',mechanisms=['sequence-note-display-normalization','sequence-note-textblock-dimensions'],sources=['display_raw_link_0','display_raw_link_1'],java=['klimt.creole.command.CommandCreoleUrl#executeAndGetRemaining','klimt.creole.legacy.StripeSimple#addUrl','klimt.creole.legacy.AtomTextUtils#createUrl (79-90)'],rust=['crates/rustuml-parser/src/parse/sequence.rs::inline_note_display','crates/rustuml-render/src/creole.rs::parse_segments_inner hyperlink arm (1142-1170)'],diagnosis='Preserving raw link escapes in the outer Display conversion is correct but insufficient. Java applies Display#getWithNewlines again to the parsed URL label and composes AtomVerticalTexts. Rust push_literal retains path\\ntext as one run. Both fresh engines lose one line and its preferred height; the measured and painted note envelope is wrong. This is a shared creole model dependency, not a font difference.',next_repair='Represent multiline hyperlink label atoms and propagate their complete dimensions; preserve the outer raw-boundary semantics.'),
 dict(id='primitive-decoration-ownership',mechanisms=['sequence-effective-arrow-color'],sources=['color_primitives_valid_0','color_primitives_valid_1','color_primitives_0','color_primitives_1'],java=['sequencediagram.command.CommandArrow ARROW_DRESSING2','skin.rose.ComponentRoseArrow#drawDressing2 (235-275)','skin.ArrowConfiguration decoration and dressing fields'],rust=['crates/rustuml-parser/src/parse/sequence.rs::parse_arrow (1363-1394)','crates/rustuml-render/src/sequence.rs arrow head dispatch and primitive emitters'],diagnosis='The first two controls used invalid ]o> ordering; raw Java exit200/error SVGs are preserved, and Rust accepts both. Corrected ]>o controls are valid but miss the Java destination ellipse. Java decoration and normal head coexist, whereas Rust selects ArrowHead::Circle instead of retaining target decoration independently. Other structural/order differences remain in the multi-primitive control. RGB/alpha01/80/FE and none materialization on emitted primitives agree in inspected strokes/fills; this does not establish coverage for the missing ellipse. No color scalar repair is inferred from an element-count mismatch.',next_repair='Preserve target decoration separately from head dressing, then derive ellipse/head paint ownership from ComponentRoseArrow. Retain primitive ordering evidence and the original grammar negatives; do not patch output coordinates.')])
put(C/'findings.json',findings)
accounts=read(R/'docs/parity-reviews/sequence-corrective-b9/maker-results/implementation-map.json')['accounts']
old_before=read(C/'cohort-before/measurements.json');old_after=read(C/'cohort-after/measurements.json')
before=read(C/'fresh-before/measurements.json')+read(C/'followup-before-complete/measurements.json')
after=read(C/'fresh-after/measurements.json')+read(C/'followup-after/measurements.json')
manifest=[]
for b,a in zip(before,after):
 assert b['source']==a['source']
 manifest.append((b,a,True))
for name in ['theme_activation_0','theme_activation_1']:
 a=next(x for x in old_after if pathlib.Path(x['source']).stem==name)
 b=next(x for x in old_before if x['source']==a['source'])
 a=dict(a,mechanism='sequence-note-textblock-dimensions',axes=['cerulean complete self preferred width','legacy versus Teoz participant and canvas envelope'])
 manifest.append((b,a,False))
paired=[]
for b,a,fresh in manifest:
 source=pathlib.Path(a['source']);source=source if source.is_absolute() else R/source
 golden=pathlib.Path(a['golden']);golden=golden if golden.is_absolute() else R/golden
 stem=('checker_ef020_' if fresh else 'checker_ef020_retained_')+source.stem
 target=R/'test-diagrams/perturbations'/a['mechanism']/stem
 target.parent.mkdir(parents=True,exist_ok=True)
 for src,dst in [(source,target.with_suffix('.puml')),(golden,target.with_suffix('.svg'))]:
  assert not dst.exists();shutil.copyfile(src,dst)
 for suffix in ['.stderr','.provenance.json']:
  src=source.with_suffix(suffix)
  if src.exists():shutil.copyfile(src,target.with_suffix(suffix))
 capture={}
 for stage,row in [('before',b),('after',a)]:
  if fresh:
   folder=('followup-before-complete' if stage=='before' else 'followup-after') if row['cohort']=='fresh4' else 'fresh-'+stage
  else:folder='cohort-'+stage
  src=C/folder/row['cli']['svg'];dst=target.with_suffix('.'+stage+'.rust.svg');shutil.copyfile(src,dst)
  dst_err=target.with_suffix('.'+stage+'.stderr');dst_err.write_text(row['cli']['stderr'])
  capture[stage]=dict(cli_exit=row['cli']['exit'],comparison_exit=row['cli_comparison']['exit'],product_library_exit=row['product_library']['exit'],assisted_library_exit=row['assisted_library']['exit'],cli_equals_library=row['cli_equals_library'],svg=str(dst.relative_to(R)),svg_sha256=sha(dst.read_bytes()))
 p=dict(source=str(target.with_suffix('.puml').relative_to(R)),golden=str(target.with_suffix('.svg').relative_to(R)),axes=a['axes'],result='pass' if a['cli_comparison']['exit']==0 else 'fail',reference_valid=a['cli_comparison']['exit']!=2,fresh=fresh,mechanism=a['mechanism'],original_source=str(source),source_sha256=sha(source.read_bytes()),golden_sha256=sha(golden.read_bytes()),measurements=capture)
 paired.append(p)
put(D/'pairs.json',paired)
commands=read(C/'build-commands.json')+read(C/'checks.json')
commands.append(dict(name='main-with-corpus',argv=['cargo','test','--locked','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','harness_guards','--','--nocapture'],exit=101,cwd=str(R),environment=read(C/'checks.json')[0]['environment'],log='main-with-corpus.log.gz',reason='Only setup-affected tiers rerun after exact pinned corpus materialization.'))
for batch,folder in [('fresh','fresh'),('followup','followup')]:
 for stage in ['before','after']:
  out=folder+'-'+stage+('-complete' if batch=='followup' and stage=='before' else '')
  commands.append(dict(name=out,argv=['uv','run','--no-project','python','docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py',str(C/('fresh-manifest.json' if batch=='fresh' else 'followup-manifest.json')),str(C/(stage+'-bin')),str(C/out)],cwd=str(R),exit=0))
put(D/'commands.json',commands)
put(D/'operational-nonzero.json',[
 dict(exit=1,operation='Initial maker file reads at prepared checkout head9e0f2cca',resolution='Switched this clean isolated checkout to exact candidate branch before review/build.'),
 dict(exit=2,operation='Exploratory Java path lookups CommunicationTileNote.java, creole/atom/AtomText.java, AtomUrl.java',resolution='Located actual CommunicationTileSelfNoteRight, legacy/AtomText and legacy/AtomTextUtils by source search; no Java execution.'),
 dict(exit=1,operation='Read checker-b9c6/verify.py',resolution='Located audit.py; no evidence fabricated.'),
 dict(exit=1,operation='First followup importer',resolution='Strict membership assertion caught relay name lacking .puml. Switched dictionary key to exact remote_name; all hashes and membership then verified.'),
 dict(exit=1,operation='First followup measurement',resolution='Manifest absent after importer assertion. Empty output directory retained.'),
 dict(exit=1,operation='Second followup measurement',resolution='Existing empty output directory refused; used distinct followup-before-complete.'),
 dict(exit=101,operation='First main tiers and guards',resolution='Empty prepared golden submodule measured zero main cases; restored exact pinned local archive and reran only affected tiers/guards.'),
 dict(exit=200,operation='Java original color_primitives_0 and1',resolution='Invalid references preserved with error SVGs and raw streams; excluded from positive passes. Corrected grammar followups independently captured.'),
 dict(exit='all captured',operation='Comparator mismatches/invalids, library panic, standing gates',resolution='Every per-case exit/stdout/stderr in cli-replays.tar.gz; every gate exit in commands.json and logs. Diagnostic invalid empty TIM panic101 is not a product CLI panic.')])
for name in ['provenance-check.json','replay-summary.json','overlap-independent.json','maker-intermediate-audit.json','golden-dependency.json','before-binaries.json','after-binaries.json','findings.json','reference-manifest.json','reference-followup-manifest.json','fresh-manifest.json','followup-manifest.json','generate.py','generate_followup.py','import_relay.py','import_followup.py','build.py','replay_checks.py','compare_cli.rs']:
 shutil.copyfile(C/name,D/name)
for name in ['reference-results.json.gz','reference-followup-results.json.gz']:
 shutil.copyfile(C/name,D/name)
for name in ['reference-transport.json','reference-followup-transport.json']:
 (D/(name+'.gz')).write_bytes(gzip.compress((C/name).read_bytes(),mtime=0))
for p in C.glob('*.log'):(D/(p.name+'.gz')).write_bytes(gzip.compress(p.read_bytes(),mtime=0))
with tarfile.open(D/'cli-replays.tar.gz','w:gz') as archive:
 for name in ['cohort-before','cohort-after','fresh-before','fresh-after','followup-before-complete','followup-after']:
  archive.add(C/name,arcname=name)
put(D/'fresh-summary.json',dict(entries=37,unique_sources=len({p['source_sha256'] for p in paired if p['fresh']}),before=dict(collections.Counter(x['cli_comparison']['exit'] for x in before)),after=dict(collections.Counter(x['cli_comparison']['exit'] for x in after)),valid_reference_count=35,invalid_reference_count=2,valid_cli_errors=sum(x['cli']['exit']!=0 and x['cli_comparison']['exit']!=2 for x in after),invalid_accepted=2,gains=sum(b['cli_comparison']['exit']==1 and a['cli_comparison']['exit']==0 for b,a in zip(before,after)),losses=sum(b['cli_comparison']['exit']==0 and a['cli_comparison']['exit']!=0 for b,a in zip(before,after)),retained_theme_copies=2))
for account in accounts:
 mechanism=account['mechanism'];ps=[p for p in paired if p['mechanism']==mechanism];fs=[f for f in findings if mechanism in f['mechanisms']]
 verdict='accepted' if all(p['result']=='pass' for p in ps) else 'rejected'
 review=dict(schema_version=1,mechanism=mechanism,reviewer=dict(task_id='thread:delegated-task:command%3Amcp%3A7a6e5142-c330-4dbe-941b-87119bcc7cef%3Adelegate-task%3A253dfb72-0e49-40b9-b53c-b6160ecef3bf',model='gpt-6-astra / high reasoning / Codex harness in T3 Code; independent checker, no children'),account_commit=account['account_commit'],implementation_commit=account['implementation_commits'][-1],implementation_commits=account['implementation_commits'],tested_revision='efc2e922bbd093cf13d827501f5e53aa850a9e68',production_revision='1b29d8c78d671649b0db6efcdc09e01391f6ce98',before_revision='5358314a0180cf760584c79bb5efd4ec1ae6320b',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',commands=['/Users/marcelo/rustuml-reference/bin/reference --cwd PRIVATE_CWD --timezone GMT+11:00 -- -pipe -tsvg (exact substituted argv, source hashes, streams and environment preserved in checker-ef020 raw relay archives)','cargo build --locked --release -p rustuml -p rustuml-oracle --bin rustuml --example diff_one (fresh distinct private targets at exact before/candidate)','uv run --no-project python docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py MANIFEST BIN_DIR PRIVATE_OUTPUT','cargo test --locked --release --no-fail-fast -p rustuml-oracle --test golden_no_oracle --test golden_pairs --test harness_guards -- --nocapture','See docs/parity-reviews/sequence-corrective-b9/checker-ef020/commands.json for all exact local commands, cwd, environment and exits.'],perturbations=ps,findings=[f['id']+': '+f['diagnosis']+' Causal next step: '+f['next_repair'] for f in fs],source_audit=fs,verdict=verdict,verdict_scope='Independent bounded source/control observation on exact candidate only. This is not whole-sequence acceptance, a shipping recommendation, a waiver of dependencies or standing red gates, or transfer of old5358314a self-cross/destroy acceptance.',residue=['Existing557 retained mismatches, fonts/deferred layout and12 accepted negative references remain active.','Parent-only326 sources were not measured in this replay.','Remote font/dot equivalence is scoped to captured environment and these sequence cases.','Scoped shadow scalar conversion and activation sibling fallback are locally supported even where complete-output dependencies reject the requested mechanism acceptance.'],evidence='docs/parity-reviews/sequence-corrective-b9/checker-ef020/README.md')
 path=R/'docs/parity-reviews'/mechanism/'review.json';assert not path.exists();put(path,review)
print('Packaged',len(paired),'pairs and seven reviews')
