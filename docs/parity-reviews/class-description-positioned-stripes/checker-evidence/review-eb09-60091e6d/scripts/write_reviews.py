# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess
from pathlib import Path
R=Path.cwd();S=Path(__file__).resolve().parent;D=R/'docs/parity-reviews/class-description-positioned-stripes/checker-evidence/review-eb09-60091e6d';F=R/'test-diagrams/perturbations/class-description-positioned-stripes/checker-60091e6d'
identity=json.load(open(D/'identity.json'));audit=json.load(open(S/'source-audit.json'));rows=json.load(open(S/'fresh-observations.json'))['rows'];pairs=json.load(open(D/'pairs.json'))
accounts=json.load(open(D/'audit.json'))['accounts'];reviews=[]
common=[
 'Independent real release CLI replay: preserved1649 990→1004 matches; maker26 2→20; zero lost passes/new valid process errors. These1675 entries contain1601 unique source hashes; smaller cohorts overlap and are not additive.',
 'Independent fresh86:36 pass,17 mismatch,2 valid-input process errors,31 invalid Java references.55 Java-valid inputs;36/55 match. The29 explicit-class controls are24 pass/5 mismatch. Original no-allowmixing controls remain preserved separately.',
 'Main exact-candidate library tiers:11188/11251 product and10523/11251 assisted, both exit101. Guards4/6, clippy20 diagnostics remain red.1444 unit and34 CLI/doc/error tests plus fmt pass. Neither main tier is a CLI headline.',
 'All prior584 independent evidence files are byte-identical. No historic account/review/pair or production/baseline/comparator/skip/target/policy was edited. Source account commit ancestry is established, but corrective-account descriptions of v1/v3 drafts prevent certifying a literal before-any-code-written chronology.',
 'No broad gate was rerun after evidence-only additions. New pairs are collected by the existing parity_perturbations test; no green standing-perturbation claim or packet acceptance. No local JVM/javac/SSH, children, native fault/production work or shipping.'
]
extras={
'class-description-positioned-stripes':['F1-mixed-family'],
'class-description-stereotype-font':[],
'class-description-stereotype-margin-envelope':[],
'class-boundary-fashion-shadow':['F2-shadow-value-input'],
'class-description-svek-envelope':['F1-mixed-family','F2-shadow-value-input','F9-group-geometry-and-inline-ownership'],
'class-description-header-envelope-ownership':['F1-mixed-family','F10-native-and-alternate-factory-residue'],
'class-command-url-named-quark':['F3-note-only-url-dropped','F4-url-smart-quotes','F5-current-group-self','F6-invalid-path-filtered','F7-root-scope-identity','F8-rooted-command-residue','F9-group-geometry-and-inline-ownership'],
'class-command-url-builder-context':['F4-url-smart-quotes','F5-current-group-self','F6-invalid-path-filtered','F7-root-scope-identity','F8-rooted-command-residue']
}
for account in accounts:
 mech=Path(account['path']).parent.name;relevant=[x for x in rows if mech in x['mechanisms']]
 # The original graphical probes without allowmixing are negative/factory controls,
 # not evidence for class geometry. Their status remains in complete census.
 perturb=[]
 for x in relevant:
  p=F/(x['name']+'.puml')
  perturb.append(dict(source=str(p.relative_to(R)),golden=str(p.with_suffix('.svg').relative_to(R)),axes=x['axes'],result='pass' if x['after']=='pass' else 'fail',classification=x['after'],java_exit=x['java_exit'],rust_exit=x['after_cli'],class_path_scope=x['cohort']=='classmode' if x['name'].startswith(('actor_','boundary_','classmode_')) else 'see raw model/factory metadata'))
 retained_names=['actor_blank','boundary_blank','actor_stereo','boundary_stereo'] if mech in ['class-description-positioned-stripes','class-description-stereotype-font','class-description-stereotype-margin-envelope','class-description-svek-envelope','class-description-header-envelope-ownership'] else ['group_left','group_top'] if 'url' in mech else []
 for x in pairs['retained_maker']:
  if x['name'] in retained_names:
   perturb.append(dict(source=x['source'],golden=x['golden'],axes=['retained maker source, not fresh','font family and symbol' if 'group' not in x['name'] else 'group note position and URL ownership'],result='fail',classification='retained mismatch',provenance=x['original_source']))
 findings=[*audit['mechanisms'][mech]['observations']]
 findings += [f"{f['id']}: {f['cause']}" for f in audit['fresh_findings'] if f['id'] in extras[mech]]
 if mech in ['class-description-stereotype-font','class-description-stereotype-margin-envelope']:
  findings.append('The four fresh class-mode stereotype wrapper controls pass, supporting the separate font and UEmpty arithmetic. Retained Serif stereotype cases remain whole-input failures. This is a rejected complete-candidate verdict because its positioned/font dependency is unsatisfied, not an allegation that the UEmpty formula itself was disproved.')
 if mech=='class-description-header-envelope-ownership':
  findings.append('Component/rectangle ownership contrasts and corrected sub/sup/size cases pass. Native styled-class controls remain unchanged before/after and fail with generic:/size metadata; no new regression is claimed. The complete dependency surface is not accepted from narrower passing observations.')
 findings+=common
 record=dict(schema_version=1,mechanism=mech,reviewer=dict(task_id=identity['reviewer_task'],model=identity['reviewer_model']),account_commit=account['commit'],implementation_commit=identity['production'],candidate_revision=identity['candidate'],before_revision=identity['before'],java_revision=identity['java_revision'],commands=[
 'CARGO_TARGET_DIR=/Users/marcelo/Library/Caches/rustuml-review-60091e6d/target-before CARGO_BUILD_JOBS=4 cargo build --release -p rustuml (at b074d2082cb9ed2b9460f79a32b170e31cc1ab5e)',
 'uv run --no-project python /Users/marcelo/Library/Caches/rustuml-review-60091e6d/run_gates.py (at exact eb09cecbc90f5690c4a09bf49118f2feff8c1c12; all exact commands and env in evidence/gates.json)',
 'uv run --no-project python /Users/marcelo/Library/Caches/rustuml-review-60091e6d/replay.py {before|after} {preserved|fresh|nonclass} (six actual invocations, raw argv in reports)',
 '/Users/marcelo/rustuml-reference/bin/reference --cwd PRIVATE_CWD --timezone GMT+11:00 -- -pipe -tsvg (parent existing Monkey Man relay only; three exact worker cwd/argv/environment manifests retained)'
 ],perturbations=perturb,findings=findings,verdict='rejected',evidence=str(D.relative_to(R)),source_audit_locations=audit['mechanisms'][mech],scope='Complete exact candidate with all retained failures; narrow supported formulas do not authorize extracted-patch acceptance.')
 path=R/'docs/parity-reviews'/mech/'review.json';assert not path.exists(),path
 path.write_text(json.dumps(record,indent=2)+'\n');reviews.append(str(path.relative_to(R)))
(D/'review-index.json').write_text(json.dumps(dict(verdict='rejected',reviews=reviews,prior_verdicts='unchanged'),indent=2)+'\n')
print('wrote',len(reviews),'independent rejected records')
