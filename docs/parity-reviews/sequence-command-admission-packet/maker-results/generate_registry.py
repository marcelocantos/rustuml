# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,re,hashlib
from pathlib import Path
C=Path(__file__).parent;J=Path('/Users/marcelo/work/github.com/plantuml/plantuml/src/main/java');rows=json.loads((C/'command-inventory-raw.json').read_text())['commands']
# ColorParser.COLORS_REGEXP, exactly as composed by simpleColor.
simple=r'#\w+[-\\|/]?\w+'
part2=r'#(?:\w+[-\\|/]?\w+;)?(?:(?:text|back|header|line|line\.dashed|line\.dotted|line\.bold|shadowing)(?::\w+[-\\|/]?\w+)?(?:;|(?![\w;:.])))+'
color='((?:'+part2+')|(?:'+simple+'))?'
handlers={0:'try_meta',2:'try_meta',5:'try_meta',7:'try_meta',9:'try_meta',12:'empty',13:'try_pragma',15:'try_skinparam',17:'skinparam_block',39:'try_hide',41:'try_activate_deactivate / try_create_destroy',43:'try_participant_decl / try_create_destroy',44:'try_participant_decl',45:'try_participant_decl',46:'try_participant_decl',48:'try_message',49:'try_external_message',50:'try_external_message',51:'try_note',52:'try_note',53:'try_note',54:'try_box',55:'try_box',56:'try_group',58:'try_return',59:'try_note',60:'note_buffer',61:'note_buffer',62:'note_buffer',63:'note_buffer',64:'try_newpage',67:'try_divider',68:'try_space',69:'try_ref',70:'ref_buffer',71:'try_autonumber',72:'try_autonumber',73:'try_autonumber',75:'try_autoactivate',76:'try_hide',77:'try_delay',78:'try_hide'}
entries=[]
for r in rows:
 i=r['registration_index'];fields=[f for f in r['fields'] if 'pattern' in f];f=next(f for f in fields if f['name'] in ('pattern','starting'));pattern=f['pattern'];ending=r.get('end_pattern') or next((x['pattern'] for x in fields if x['name']=='ending'),None)
 r['rust_handler']=handlers.get(i);r['rust_support']='existing partial handler; unmatched variants produce a warning' if i in handlers else 'unimplemented in sequence parser; class warning, no fabricated event'
 path=J/(r['class'].split('$')[0].replace('.','/')+'.java');r['java_source']=str(path);r['java_source_sha256']=hashlib.sha256(path.read_bytes()).hexdigest()
 if i==56:
  # CommandGrouping's three negative fixed lookbehinds depend only on TYPE.
  pattern=r'^(&[%s]*)?(?:(opt|alt|loop|par|par2|break|critical|group|partition)(#\w+)?|(else|end|also))(?:[%s]+(#\w+))?(?:[%s]+(.*?))?$'
 if '(?!' in pattern:
  assert color in pattern,(i,pattern)
  pattern=pattern.replace(color,'{COLOR}')
 assert '(?<' not in pattern
 mode='Brace' if i==17 else ('End' if ending else 'Single')
 # Plain color fields in the sequence commands call HColorSet#getColor.
 # Preserve the source capture positions by naming (not adding) these groups.
 n=0
 if i>=40:
  def name_color(m):
   global n
   name='seq_color'+str(n);n+=1
   return '(?P<'+name+r'>#\w+)'
  pattern=re.sub(re.escape(r'(#\w+)'),name_color,pattern)
 if i in (48,49,50):
  marker=r"((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)"
  k=0
  while marker in pattern:
   start=pattern.index(marker)
   pattern=pattern[:start]+'(?P<seq_style'+str(k)+'>'+pattern[start+1:]
   k+=1
 entries.append(f'''        // {i}: {r['class']}
        RegisteredSequenceCommand::new("{r['class'].split('.')[-1]}", r###"{pattern}"###, {int(f.get('fox_signature','0'))}u64,
            CommandEnd::{mode}'''+(f'(registered_regex(r###"{ending}"###))' if ending else '')+f''', {str(i in handlers).lower()}),''')
(C/'inventory.json').write_text(json.dumps(rows,indent=2)+'\n')
code='''// SequenceDiagramFactory#initCommandsList / CommonCommands#addCommonCommands1,
// pinned 71806a23. Registration order and exact original patterns are preserved
// in sequence-registered-command-rejection/inventory.json (81-command source
// probe). These are source grammars, not fixture data. Account 3b38384b3.
static REGISTERED_SEQUENCE_COMMANDS: LazyLock<Vec<RegisteredSequenceCommand>> = LazyLock::new(|| {
    vec![
'''+ '\n'.join(entries)+'''
    ]
});
'''
(C/'registry-table.rs').write_text(code)
