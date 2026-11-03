# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Preserve maker measurements without editing the frozen acceptance corpus."""
from pathlib import Path
import subprocess,json,shutil,tarfile,hashlib,collections,re
root=Path.cwd();scratch=Path('/tmp/rustuml-cff2');account=root/'docs/parity-reviews/class-command-grammar-closure';out=account/'maker-evidence';out.mkdir()
def copy(src,dest):
 dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(src,dest)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def load(p):return json.loads(p.read_text())
for group in ['loss36','checker23','prior31','theme33','fresh']:
 shutil.copytree(scratch/group,out/'inputs'/group)
 for phase in ['before','after']:shutil.copytree(scratch/phase/group,out/phase/group)
for phase in ['before','after']:
 copy(scratch/(phase+'833')/'results.json',out/(phase+'-833-results.json'))
 # Every SVG, AST and diff from the full corpus remains recoverable. Existing
 # source/reference bytes remain in the frozen corpus, identified by hashes.
 with tarfile.open(out/(phase+'-833-outputs.tar.gz'),'w:gz') as archive:
  archive.add(scratch/(phase+'833'),arcname=phase+'833')
shutil.copytree(scratch/'final-validation',out/'validation')
shutil.copytree(scratch/'java-diagnostics',out/'java-diagnostics')
shutil.copytree(scratch/'negative-controls',out/'negative-controls')
for name in ['rustuml-cff2-inventory.py','rustuml-cff2-measure.py','rustuml-cff2-fresh.py','rustuml-cff2-final-verify.py','rustuml-cff2-preserve.py']:
 copy(Path('/tmp')/name,out/'scripts'/name)
for n in ['check_cli.py','check_svg.rs']:
 copy(root/'docs/parity-reviews/class-native-container-factory-precedence/checker-evidence'/n,out/'scripts'/n)
for kind in ['no_oracle','golden']:
 copy(root/'test-diagrams'/(kind+'_failure_names.txt'),out/'validation'/('after-'+kind+'-failure-names.txt'))
 copy(Path('/tmp/rustuml-cff-final-'+kind+'_failure_names.txt'),out/'validation'/('before-'+kind+'-failure-names.txt'))
# Keep parent review bytes as supporting input, not an authored attestation.
# They remain outside this account's checker slot; only hashes/commit pointers
# are recorded here and the reports themselves stay untouched in their commits.
before=load(out/'before-833-results.json');after=load(out/'after-833-results.json')
assert len(before)==len(after)==833
changes=[{'source':a['source'],'before':b['status'],'after':a['status']} for b,a in zip(before,after) if a['status']!=b['status']]
(out/'833-status-changes.json').write_text(json.dumps(changes,indent=2)+'\n')
cohorts={}
for cohort in sorted({a['cohort'] for a in after}):
 cohorts[cohort]={phase:dict(collections.Counter(r['status'] for r in rows if r['cohort']==cohort)) for phase,rows in [('before',before),('after',after)]}
groups={}
for group in ['loss36','checker23','prior31','theme33','fresh']:
 groups[group]={}
 for phase in ['before','after']:
  rows=load(out/phase/group/'results.json');groups[group][phase]={'total':len(rows),'strict_matches':sum(not (r['cli_exit'] or r['ast_exit'] or r['comparison_exit']) for r in rows.values()),'parse_errors':[n for n,r in rows.items() if r['ast_exit']],'mismatches':[n for n,r in rows.items() if r['comparison_exit']]}
account_files=subprocess.check_output(['git','diff','--name-only','0a04a06a','HEAD'],text=True).splitlines()
accounts=[]
for f in account_files:
 if f.endswith('/account.json'):
  first=subprocess.check_output(['git','log','--reverse','--format=%H','0a04a06a..HEAD','--',f],text=True).splitlines()[0];accounts.append({'path':f,'original_preimplementation_commit':first,'sha256':sha(root/f)})
source_hashes={str(p.relative_to(out)):sha(p) for p in (out/'inputs').rglob('*') if p.is_file()}
model_inventory={}
for group in groups:
 model_inventory[group]={}
 for ast in (out/'after'/group).glob('*.ast.txt'):
  txt=ast.read_text();model_inventory[group][ast.name]={'factory':txt.split('(',1)[0].strip(),'class_entities':txt.count('ClassEntity {'),'packages':txt.count('Package {'),'notes':txt.count('Note {'),'relationships':txt.count('Relationship {')}
(out/'model-inventory.json').write_text(json.dumps(model_inventory,indent=2)+'\n')
main_changes={}
for kind in ['no_oracle','golden']:
 a=set((out/'validation'/('before-'+kind+'-failure-names.txt')).read_text().splitlines());b=set((out/'validation'/('after-'+kind+'-failure-names.txt')).read_text().splitlines());main_changes[kind]={'repaired':sorted(a-b),'regressed':sorted(b-a)}
impl=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
report={
 'schema_version':1,'role':'implementation maker; not an independent checker','status':'pending independent review; relevant residual failures remain blocking','implementation_commit':impl,'before_commit':'0a04a06acb2a072946356a741a5850377dd5c039',
 'accounts':accounts,'account_schema_projections':['486c9fe9ecf4b85f4578a41a6e1ed018a03a1d4f','ee458fb064d859d2a1fdba9e12fc8dd0759cba79'],
 'causal_model':[
  'One consuming command engine retains quote/body ownership and ordered mixing state. Boundary leaves, action/process/queue containers, package modifier slots and parenthesis endpoints now produce ordinary model objects. Complete description fallback consumes its own grammar after class rejection; empty package-only models survive factory selection.',
  'Ordinary package command precedence is resolved before explicit USymbol grammar. Aliases, stereotype slots, tags, URLs and reopened identities are retained. Named notes register scoped quarks, share endpoint lookup and retain declaration membership independently of their target.',
  'Mixed component and boundary images use their Java USymbol margins/painting. Current spot style overrides stale skinparam palette in both repeated-theme orders. Chrome carries command alignment and bordered text-block dimensions.',
  'Serialized SVEK positions cross the two-decimal boundary once. Measured cloud/rectangle/polygon frontiers determine one common painted origin for every group, leaf, note and spline. Horizontal note links enter the early lines0 stream. Cluster title shape padding applies after title/stereotype merging.',
  'Group endpoints use existing SVEK point nodes, compound clipping, precise cubic shortening and folder magnetic borders. Parenthesis arcs and SVG identity follow LinkType/Extremity semantics.'
 ],
 'prepared_833':{'denominator':833,'composition':'810 original + 23 class checker sources','before':dict(collections.Counter(r['status'] for r in before)),'after':dict(collections.Counter(r['status'] for r in after)),'cohorts':cohorts,'repaired_comparisons':sum(b['status']!='pass' and a['status']=='pass' for b,a in zip(before,after)),'previously_passing_regressions':[a['source'] for b,a in zip(before,after) if b['status']=='pass' and a['status']!='pass'],'valid_parse_errors_before':sum(r['status']=='process_error' for r in before),'valid_parse_errors_after':sum(r['status']=='process_error' for r in after),'details':'maker-evidence/833-status-changes.json','all_outputs':'maker-evidence/{before,after}-833-outputs.tar.gz'},
 'targeted_groups':groups,
 'main_tiers':{'eligible':11251,'total_sources':12550,'frozen_excluded':1299,'source_only':{'before_pass':11181,'after_pass':11188,'after_mismatch':63},'assisted':{'before_pass':10497,'after_pass':10497,'after_mismatch':754},'case_deltas':main_changes,'exit':101,'reason':'Broad residual mismatches and frozen improvement ratchets; no baselines updated.'},
 'unit_tests':{'command':'cargo test --workspace --lib','before_pass':1420,'after_pass':1425,'after_parser':571,'after_renderer':772,'after_layout':41,'after_math':10,'after_oracle':31,'failed':0,'exit':0},
 'other_checks':{'guards':'4/6; decimal ratchet and parity review evidence fail; baselines and rejected reviews untouched','clippy':'exit 101: 21 library / 22 library-test diagnostics at existing lint sites; no unrelated lint edits','format':'rustfmt --check for six changed production files: exit 0','diff_check':'git diff --check: exit 0','perturbation_harness':'cargo test --release -p rustuml-oracle --test parity_perturbations: 1 control passed, held-out comparison test failed; full log retained','commands':'maker-evidence/validation/commands.json'},
 'remaining_targeted_counterexamples':[
  {'source':'test-diagrams/perturbations/class-native-container-mixed-leaf-precedence/package_target_parenthesis_one_dash_lr.puml','copied_input':'maker-evidence/inputs/loss36/27-package_target_parenthesis_one_dash_lr.puml','dependency':'LR one-rank group endpoint solve; accepted endpoints remain in AST but exact layout diverges. Existing LayoutGraph SVEK point/cluster/early-edge implementation needs further trace. No geometry tuning or shared layout edits.'},
  {'source':'test-diagrams/perturbations/class-svek-node-paint-order/followup3_upper_interface_bare_lr.puml','copied_input':'maker-evidence/inputs/loss36/35-followup3_upper_interface_bare_lr.puml','dependency':'Description route retains component/interface, but shielded interface differs in x and edge controls. Java emits the real three-row HTML shield; LayoutGraph::svek_shielded_node_table uses a rounded single-cell approximation and custom center port. Further shared layout ownership is needed; no half-pixel renderer patch.'},
  {'copied_input':'maker-evidence/inputs/fresh/group_lr.puml','dependency':'Fresh valid LR group-to-group parenthesis case still mismatches. Java DOT/SVG and strict diff retained. It blocks acceptance despite repaired TB group routes.'}
 ],
 'negative_controls':{'mixing_late':'Both before and after continuation reject with CLI/AST exit 1. Java-invalid preserved reference remains unchanged.','modifier_mandatory':'Original *class negative still exits 0 as Deployment; existing unresolved negative retained outside positive denominators.','legacy_unit_color_order':'One inline renderer unit used Java-invalid color-before-stereotype package syntax. Original full source and Java error SVG preserved under negative-controls/unit_color_order_full; test now uses valid stereotype-before-color syntax with the same intended geometry assertions. No reference/fixture was rewritten.'},
 'theme_dependency':{'review_commit':'b2c9143f95de43cde37cbcc055a56da40e56735e','review_path':'docs/parity-reviews/tim-theme-post-expansion-source-identity/review.json','copied_review_sha256':sha(scratch/'theme-review.json'),'parent_sources':33,'invalid_reference':'checker_inline_comment','valid_reference_denominator':32,'strict_matches_before':5,'strict_matches_after':7,'repaired':['checker_repeated_reverse','maker_repeated_themes'],'remaining':'Other TIM/source-identity/rendering failures remain. Two parse errors (checker_dump_memory, checker_inner_comment) predate this packet.','include_caveat':'checker_include_theme uses a parent-relative docs include. Parent T14.6.2 tracks cwd dependence. Parent support docs are not integrated in this worktree; its raw mismatch is preserved, not claimed as a geometry failure.','other_workers':'Preprocessing maker a3082894f29e642f146bf2bc573e33df39753e51 / identity candidate 0f2fb90c were not imported or edited.'},
 'integration_caveats':[
  'Parent has 913 sources = 810 original + 23 class + 41 sequence + 33 theme + 6 registry controls. This worktree executes the prepared 833 plus a separately copied 33-theme cohort (866 observed sources), not the full parent 913. Fresh 31 maker cases are a separate diagnostic matrix, not checker-held-out acceptance inputs.',
  'Parent accepted registry fix 22cd748c2ea2b5880c1dc5e4f7796c81a950d08b and prerequisites c6ba01b4453825dd6ec85f849964716ad5fcd961 / 43f6e7b82c52c6b4ce989926e33f84eec84bcc38 were not cherry-picked. Their accepted review bc9e9463 was read. The two genuine sequence panics in the omitted sequence cohort remain parent failures, not repaired here.',
  'All 277 valid mismatches and 7 invalid references in the prepared 833 remain executable failures; no metric scope waiver or acceptance claim. Full isolated case outputs and source/reference hashes are preserved.',
  'No preprocessing, shared font/style/layout/vendor, harness, CI, bullseye, old review/account/fixture/reference or frozen comparator/slack/exclusion/baseline edits. component.rs only exposes existing compound clipping to class.rs. ascii_class.rs only updates an inline model constructor.',
  'New accounts were committed before their production changes. Later schema-only commits project unchanged planned text into required keys without replacing original historical account contents. The final common-origin account supersedes the earlier per-owner paint-origin plan.',
  'JAR is source-pinned rebuilt output, not historical byte-identical. Font and Graphviz historical equivalence remain unproven. No push, PR, release, child agent or checker attestation.'
 ],
 'reference':{'java_revision':'71806a23780b04a5ccde2f8ceb5121edad5eb711','jar_sha256':sha(Path('/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar')),'java':'/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java','stdlib_revision':'747d1eae809cde20675976d67e3eb84dc0021a0c','golden_gitlink':'9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a','before_binary_sha256':sha(Path('/tmp/rustuml-cff2-before-rustuml')),'comparator_source':'docs/parity-reviews/class-native-container-factory-precedence/checker-evidence/check_svg.rs','comparator_module_sha256':sha(root/'crates/rustuml-oracle/src/compare.rs'),'GEOM_EPS':0.02},
 'source_reference_manifest':source_hashes,
 'evidence_kind':'Maker results only. Parent must assign a fresh independent checker; these are not acceptance attestations.'
}
# Resolve the two loss36 sources from frozen hashes instead of trusting inferred
# directory names; repeated basenames may belong to different mechanisms.
for row in report['remaining_targeted_counterexamples']:
 if 'source' in row:
  p=account/row['copied_input'];digest=sha(p)
  row['source']=next(a['source'] for a in after if a['source_sha256']==digest)
(account/'maker-results.json').write_text(json.dumps(report,indent=2)+'\n')
manifest={str(p.relative_to(out)):sha(p) for p in out.rglob('*') if p.is_file()}
(out/'manifest-sha256.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('preserved',len(manifest),'files; report',account/'maker-results.json')
