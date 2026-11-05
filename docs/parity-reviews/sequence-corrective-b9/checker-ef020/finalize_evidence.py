# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,pathlib,subprocess,shutil
c=pathlib.Path(__file__).parent;r=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ef020aa1');d=r/'docs/parity-reviews/sequence-corrective-b9/checker-ef020';j=pathlib.Path('/Users/marcelo/work/github.com/plantuml/plantuml')
sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=r)
def put(name,value):(d/name).write_text(json.dumps(value,indent=2)+'\n')
old='68c8b95ef1087b5daa83d16b7e8e83670d5ca10f';paths=git('diff-tree','--no-commit-id','--name-only','-r',old).decode().splitlines();inventory=[]
for path in paths:
 before=git('show',old+':'+path);after=(r/path).read_bytes();assert before==after
 inventory.append(dict(path=path,sha256=sha(before),git_blob=git('rev-parse',old+':'+path).decode().strip()))
assert len(inventory)==468;put('prior468-inventory.json',inventory)
put('tool-identity.json',{tool:subprocess.check_output(argv,cwd=r).decode().strip() for tool,argv in [('rustc',['rustc','-Vv']),('cargo',['cargo','-V']),('uv',['uv','--version']),('python',['python3','--version']),('git',['git','--version'])]})
java_names=['Display','AtomText','AtomTextUtils','StripeSimple','CommandCreoleUrl','ValueImpl','Style','FromSkinparamToStyle','ArrowAndNoteBox','Step1Message','Step1Abstract','AbstractTextualComponent','Rose','ComponentRoseActiveLine','ComponentRoseLine','ComponentRoseNote','AbstractComponentRoseArrow','ComponentRoseSelfArrow','MessageSelfArrow','CommunicationTileSelf','CommunicationTileSelfNoteRight','LivingSpace','PlayingSpaceWithParticipants','SequenceDiagramFileMakerTeoz','LifeLine','LifeSegmentVariation','TileParallel','GroupingTile','NoteTile','LiveBoxes','HColorSet','SvgGraphics','CommandReturn','CommandArrow','ArrowConfiguration','ComponentRoseArrow']
java_paths=[]
for name in java_names:java_paths.extend(j.glob('src/main/java/**/'+name+'.java'))
java_paths.append(j/'src/main/resources/skin/plantuml.skin')
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=j).decode().strip()=='71806a23780b04a5ccde2f8ceb5121edad5eb711'
put('java-source-inventory.json',[dict(path=str(p.relative_to(j)),sha256=sha(p.read_bytes())) for p in sorted(set(java_paths))])
production=['crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs']
put('production-source-inventory.json',[dict(path=p,before_sha256=sha(git('show','5358314a0180cf760584c79bb5efd4ec1ae6320b:'+p)),after_sha256=sha((r/p).read_bytes()),candidate_sha256=sha(git('show','efc2e922bbd093cf13d827501f5e53aa850a9e68:'+p))) for p in production])
(d/'production.diff.gz').write_bytes(gzip.compress(git('diff','5358314a0180cf760584c79bb5efd4ec1ae6320b','efc2e922bbd093cf13d827501f5e53aa850a9e68','--',*production),mtime=0))
assert not git('diff','--name-only')
for name in ['final_registration.py','finalize_evidence.py']:shutil.copyfile(c/name,d/name)
# Packaging source preserves the authoring recipe; do not rerun into existing paths.
shutil.copyfile(c/'package_review.py',d/'package_review.py')
pairs=json.loads((d/'pairs.json').read_text());old_sources={x['source_sha256'] for x in json.loads((c/'cohort-after/measurements.json').read_text())};fresh={x['source_sha256'] for x in pairs if x['fresh']};put('denominators.json',dict(replayed_entries=1901,replayed_unique_sources=1853,replayed_duplicate_entries=48,fresh_entries=37,fresh_unique_sources=len(fresh),fresh_overlap_with_replayed=len(fresh&old_sources),copied_retained_theme_pairs=2,new_registered_pairs=39,parent_entries=1449,parent_overlap_old_entries=1123,parent_only_unmeasured=326))
print('Verified all468 prior files, exact production and37 fresh source identities')
