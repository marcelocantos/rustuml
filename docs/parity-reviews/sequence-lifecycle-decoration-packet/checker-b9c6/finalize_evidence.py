"""Assemble source audit and verify the independent evidence without changing product code."""
import gzip
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path('/Users/marcelo/.t3/worktrees/rustuml/t3-b9c6cf04')
CACHE = Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-b9c6cf04')
OUT = ROOT / 'docs/parity-reviews/sequence-lifecycle-decoration-packet/checker-b9c6'
JAVA = Path('/Users/marcelo/work/github.com/plantuml/plantuml')
JAVA_REV = '71806a23780b04a5ccde2f8ceb5121edad5eb711'
CANDIDATE = '5358314a0180cf760584c79bb5efd4ec1ae6320b'

def git(*args, cwd=ROOT):
    return subprocess.check_output(['git', *args], cwd=cwd)

def digest(data):
    return hashlib.sha256(data).hexdigest()

def write_json(name, value):
    (OUT / name).write_text(json.dumps(value, indent=2) + '\n')

java_files = '''sequencediagram/AbstractMessage.java
sequencediagram/SequenceDiagram.java
sequencediagram/command/CommandArrow.java
sequencediagram/command/CommandReturn.java
sequencediagram/graphic/DrawableSetInitializer.java
sequencediagram/graphic/MessageSelfArrow.java
sequencediagram/graphic/MessageArrow.java
sequencediagram/graphic/LifeDestroy.java
sequencediagram/graphic/LifeLine.java
sequencediagram/graphic/ArrowAndNoteBox.java
sequencediagram/graphic/NoteBox.java
sequencediagram/teoz/CommunicationTile.java
sequencediagram/teoz/CommunicationTileSelf.java
sequencediagram/teoz/GroupingTile.java
sequencediagram/teoz/TileParallel.java
sequencediagram/teoz/LifeEventTile.java
sequencediagram/teoz/LiveBoxes.java
skin/rose/Rose.java
skin/rose/ComponentRoseArrow.java
skin/rose/ComponentRoseSelfArrow.java
skin/rose/ComponentRoseDestroy.java
skin/rose/ComponentRoseParticipant.java
skin/rose/ComponentRoseNote.java
skin/rose/ComponentRoseActiveLine.java
klimt/color/ColorTrieNode.java
klimt/color/HColorSet.java
command/note/sequence/FactorySequenceNoteOnArrowCommand.java'''.splitlines()
assert git('rev-parse', 'HEAD', cwd=JAVA).decode().strip() == JAVA_REV
java_hashes = {}
for path in java_files:
    rel = 'src/main/java/net/sourceforge/plantuml/' + path
    current = (JAVA / rel).read_bytes()
    assert current == git('show', f'{JAVA_REV}:{rel}', cwd=JAVA), rel
    java_hashes[rel] = digest(current)

audit = {
    'schema_version': 1,
    'candidate': CANDIDATE,
    'java_revision': JAVA_REV,
    'java_read_only_source_hashes': java_hashes,
    'chronology': {
        'evidence': 'audit.json',
        'accounts_precede_all_implementations': True,
        'refinements_precede_relevant_implementations': True,
        'refinements': {
            '42ec7126f6559d832d7ba5bbb9c293d152ca8afa': 'x-- is forward starting CROSSX with reverseDefine=false; <-x is reverse. The erroneous initial hypothesis is retained and explicitly corrected before code.',
            '09eec925d59a83a461606ca26b7138f903ee999e': 'Compare complete ordinary note-event envelope against existing theme/component bounds; remove theme exclusion.',
            '14228d60539c365b92f96d3051c99cdf05f22e2c': 'Note preferred width already includes shadow; carry its domain through final margin, avoiding a second shadow.'
        },
        'candidate_vs_production_crates_diff': 'empty',
        'ascii_scope': 'Exactly two test Message constructors gained parallel:false; no ASCII production behavior changed.'
    },
    'command_dataflow': [
        'Message.parallel and ReturnMessage.parallel each use serde(default). Standalone serde-defaults-command.json probe against candidate rlibs proves missing fields deserialize false and true survives roundtrip.',
        'SequenceDiagram.activate owns activationState independently of graphical variation acceptance; ACTIVATE pushes, DEACTIVATE pops, DESTROY leaves command return ownership intact. AbstractMessage.addLifeEvent rejects same-participant ACTIVATE after a close on the same owning message; a different participant is not forbidden by that check. Rust resolve_sequence_returns carries its separate owner/stack/closed map before painting.',
        'CommandReturn takes the command stack or internal last message, reverses endpoints and carries arrow configuration; PARALLEL is applied to an internal Message but not MessageExo. Rust preserves this branch. Fresh inbound/external return controls isolate downstream colors from ownership.',
        'HColorSet accepts 1/3/6/8 hex digits and the exact 157-name/symbol domain checked in color-domain-audit.json. Unknown return colors reject. Valid colors remain a rendering dependency and fail six fresh cases; validation is not color serialization.',
        'AbstractMessage.isParallelWith is directional ancestry. Rust stores prior parallel message identity, rather than making row membership symmetric. The graphical contact and predecessor composition still fail B9-PARALLEL-CONTACT and B9-PARALLEL-PREDECESSOR.'
    ],
    'constants_and_branch_provenance': [
        {'rust': 'SELF_CROSS_SPACE=6', 'java': 'skin.rose.ComponentRoseArrow.spaceCrossX=6', 'assessment': 'Direct named source constant. Starting/final CROSSX branches are arrow dressing semantics, not fixture direction guesses.'},
        {'rust': 'TEOZ_DEACTIVATE_CALLBACK_STAGGER=5', 'java': 'sequencediagram.teoz.LiveBoxes.addStep', 'assessment': 'Direct collision increment restricted to DEACTIVATE, not arbitrary row separation.'},
        {'rust': 'DESTROY_CROSS_HALF=9; preferred dimension=2*half', 'java': 'skin.rose.ComponentRoseDestroy.crossSize and getPreferredHeight/getPreferredWidth', 'assessment': 'Direct 18-unit component allocation. Attached legacy self owner contributes its component end-level loop drop; standalone LifeDestroy starts at freeY.'},
        {'rust': 'self cross 2*spaceCrossX; spaceCrossX/2; ARROW_SIZE/2; stroke-width:2; reverse x1+1', 'java': 'skin.rose.ComponentRoseSelfArrow.drawRightSide/drawLeftSide', 'assessment': 'Algebraic translations of component local coordinates: both diagonals use solid UStroke.withThickness(2) independent of dotted body. Reverse x1 restores the body endpoint decrement before positioning the decoration. Eight fresh cross controls pass across engines/direction/closing depth; preserved zero/deeper depth and lowered-return controls also retained.'},
        {'rust': 'created head +1 + shadow; max(head_height, arrow_height)', 'java': 'ComponentRoseParticipant.getPreferredHeight = textHeight+deltaShadow+1+getDeltaCollection; CommunicationTile.getPreferredHeight', 'assessment': 'The one is a source preferred-height term, not a fitted canvas increment. Existing participant box geometry holds collection extras. This is not proof of all participant shapes or styles.'},
        {'rust': 'level*ACTIVATION_HALF_W; (level-2)*ACTIVATION_HALF_W; (2-level)*ACTIVATION_HALF_W', 'java': 'CommunicationTile.LIVE_DELTA_SIZE=5; drawU posB/posD endpoint and created-head placement', 'assessment': 'Direction and active-depth equations extend to creating endpoints. Zero/nonzero tests distinguish existing activation state. No ladder keyed to specific depth counts.'},
        {'rust': 'max(preferred arrow width, actual endpoint span)+note component width+right shift', 'java': 'graphic.ArrowAndNoteBox.getPreferredWidth; MessageArrow; NoteBox.getRightShift', 'assessment': 'Right shift samples activation at component start, before arrow-contact activation. Reused twice-padding/arrow-half terms convert existing event ordinate to component start. Display normalization remains a required input and fails escaped multiline.'},
        {'rust': 'CENTER margin difference ROSE_NOTE_MARGIN_X2-NOTE_TEXT_X_PAD =15-6', 'java': 'ComponentRoseNote constructor and AbstractTextualComponent.getTextWidth', 'assessment': 'Direct source margin difference, not text-dependent tuning. Explicit noteTextAlignment does not reach this value; numeric scoped shadow also fails. Correct equations alone do not satisfy styled note dimensions.'},
        {'rust': 'max_message_note_right comparison and one-time shadow correction', 'java': 'DrawableSetInitializer.prepareMissingSpace; ComponentRoseNote.getPreferredWidth', 'assessment': 'Dominance is determined by complete graphical event extent, not a theme-name exception. Preferred note width already includes deltaShadow; subtraction at the final margin conversion avoids double counting. Shared activation style and Display input failures remain in scope.'},
        {'rust': 'new create/destroy callback consumers reuse -2 ordinate conversion', 'java': 'Rose component margins and existing Rust teoz_message_y_offset/draw_activation_bar coordinate convention', 'assessment': 'Inherited coordinate convention, not an independently universal two-pixel law. Adjacent comments attribute Rose marginY*2. Parallel contacts prove the shared ordinate abstraction insufficient; reject scheduling closure rather than treating existing reuse as proof.'},
        {'rust': 'Teoz destroy stroke:#A80036;stroke-width:2', 'java': 'ComponentRoseDestroy draws with merged style.getColorLine/getStroke', 'assessment': 'Duplicates the existing legacy default, not Java arbitrary-style resolution. Adjacent default-color description is incomplete provenance for styled destruction. Teoz is rejected; legacy acceptance is confined to ordinate/allocation, with style dependency retained.'},
        {'rust': 'maximum==0 destroy-only stairs; level-1 destroy indent; neutral CREATE; first relevant event destroy scan', 'java': 'LiveBoxes.drawBoxes/drawOneLevel/isNextEventADestroy and LifeEventType', 'assessment': 'Structural source branches, not fixture-shaped cases. Splitting segment and destroy vectors loses Java per-level paint order: B9-DESTROY-ORDER rejects scheduling.'},
        {'rust': 'parser hex lengths 1|3|6|8; default false; owner external sentinels [ and ]', 'java': 'HColorSet.parseSimpleColor; Message/MessageExo; CommandReturn', 'assessment': 'Language/model constants, checked directly. 157 registered names/symbols match exactly. Boolean defaults are serde compatibility, not alternate ASCII behavior.'}
    ],
    'fixture_logic_audit': {
        'scope': 'All four production files changed by packet; renderer diff plus parser/model changes, with implementation chronology and refinements.',
        'new_fixture_label_conditions': False,
        'new_shipping_golden_reads_or_embeds': False,
        'new_branch_count_constant_ladders': False,
        'new_coordinate_keyed_svg_surgery': False,
        'generic_created_head_group_unwrapping': 'Moved pre-existing component wrapper removal; not introduced as a counterexample-specific patch.',
        'new_metrics_table_or_high_precision_layout_values': False,
        'limits': 'Does not certify unrelated historical renderer code. Default destroy style provenance remains incomplete; known decimal guard remains red.'
    },
    'findings': 'source-findings.json',
    'dependencies': 'Full candidate only; shared color, font, entity, style, note Display, lifecycle geometry and file-reader/cwd dependencies remain active. No extracted or recomposed packet acceptance.',
}
write_json('source-audit.json', audit)
shutil.copy2(CACHE / 'package_review.py', OUT / 'package_review.py')
shutil.copy2(__file__, OUT / 'finalize_evidence.py')

mechanisms = json.loads((OUT / 'summary.json').read_text())['mechanisms']
checks = []
sources = set()
for mechanism in mechanisms:
    path = ROOT / 'docs/parity-reviews' / mechanism / 'review.json'
    record = json.loads(path.read_text())
    assert record['schema_version'] == 1
    assert record['mechanism'] == mechanism
    assert record['tested_revision'] == CANDIDATE
    assert record['java_revision'] == JAVA_REV
    for field in ('account_commit', 'implementation_commit', 'java_revision'):
        assert len(record[field]) == 40
    assert len(record['commands']) > 0 and len(record['findings']) > 0
    assert record['reviewer']['task_id'] and record['reviewer']['model']
    assert len(record['perturbations']) >= 2
    for case in record['perturbations']:
        source, golden = ROOT / case['source'], ROOT / case['golden']
        assert source.with_suffix('.svg') == golden
        assert digest(source.read_bytes()) == case['source_sha256']
        assert digest(golden.read_bytes()) == case['golden_sha256']
        assert len(case['axes']) >= 2
        assert case['result'] in ('pass', 'fail')
        if record['verdict'] == 'accepted':
            assert case['result'] == 'pass'
        for key in ('rust_output', 'before_output'):
            assert (ROOT / case[key]).exists()
        assert source.with_suffix('.provenance.json').exists()
        assert source.with_suffix('.java.stderr').exists()
        assert source not in sources
        sources.add(source)
    checks.append({'mechanism': mechanism, 'cases': len(record['perturbations']), 'verdict': record['verdict']})
assert len(sources) == 49
assert sources == {p for mechanism in mechanisms for p in (ROOT / 'test-diagrams/perturbations' / mechanism).glob('*.puml')}
assert not git('diff', '--name-only', CANDIDATE).strip(), 'Existing tracked file changed'
assert git('rev-parse', 'HEAD').decode().strip() == CANDIDATE
for name, expected in [
    ('reference-results.json.gz', '55564eb56482001915e1c72b58b46373d2dea1b3be61853a640ef0eef7566081'),
    ('supplement-results.json.gz', '482551b1dbac55b80a242e697981c24136709b36130084335c2cc44975faac13'),
]:
    data = (OUT / name).read_bytes()
    assert digest(data) == expected
    json.loads(gzip.decompress(data))
write_json('evidence-validation.json', {'checks': checks, 'pairs': 49, 'tracked_existing_file_changes': [], 'java_source_files_verified_against_pinned_git_blob': len(java_hashes), 'raw_remote_gzip_hashes_verified': True})
print('Source audit and evidence schema/path/hash checks passed for all49 pairs.')
