# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,hashlib,json,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();before=[];after=[];xml=[]
for batch in ['main','supplement']:
    for side,dest in [('before',before),('after',after)]:
        d=C/('fresh-'+batch+'-'+side);rows=json.loads((d/'measurements.json').read_text());dest.extend(rows)
        for r in rows:
            p=d/r['cli']['svg'];raw=p.read_bytes()
            if raw:
                try:root=ET.fromstring(raw);xml.append(dict(source=r['source'],side=side,valid=True,family=root.attrib.get('data-diagram-type')))
                except ET.ParseError as e:xml.append(dict(source=r['source'],side=side,valid=False,error=str(e)))
def counts(rows):
    c=collections.Counter()
    for r in rows:
        c['nonpositive' if not r['observed_valid'] else 'valid_error' if r['cli']['exit'] else 'match' if r['cli_comparison']['exit']==0 else 'mismatch']+=1
    return dict(entries=len(rows),**c)
gains=[y['source'] for x,y in zip(before,after) if x['cli_comparison']['exit']!=0 and y['cli_comparison']['exit']==0]
losses=[y['source'] for x,y in zip(before,after) if x['cli_comparison']['exit']==0 and y['cli_comparison']['exit']!=0]
errors=[dict(source=y['source'],source_sha256=y['source_sha256'],diagnostic=y['cli']['stderr']) for x,y in zip(before,after) if x['cli']['exit']==0 and y['cli']['exit'] and y['observed_valid']]
result=dict(before=counts(before),after=counts(after),gains=gains,lost_matches=losses,new_valid_errors=errors,lost_rejections=[y['source'] for x,y in zip(before,after) if x['cli']['exit'] and not y['cli']['exit']],panics=[x['source'] for x in after if x['cli']['exit'] not in (0,1)],timeouts=[x['source'] for x in after if x['cli']['exit']==124],strict_xml=dict(outputs=len(xml),failures=[x for x in xml if not x['valid']]),mechanisms={m:dict(before=counts([r for r in before if r['mechanism']==m]),after=counts([r for r in after if r['mechanism']==m])) for m in sorted({r['mechanism'] for r in after})})
(C/'fresh-summary.json').write_text(json.dumps(result,indent=2)+'\n');(C/'fresh-xml.json').write_text(json.dumps(xml,indent=2)+'\n');(C/'fresh-after.json').write_text(json.dumps(after,indent=2)+'\n');(C/'fresh-before.json').write_text(json.dumps(before,indent=2)+'\n')
# Preserve mismatched intentions, with source-derived explanation rather than
# rewriting the generator's declared intention.
disagreements=[]
for r in after:
    if r['intended_valid']==r['observed_valid']:continue
    name=r['name'];reason=''
    if r['java_exit']==0 and not r['observed_valid']:reason='HColorAutomagic is syntactically/execution-valid but rendering invokes unsupported HColor.toColor; Java returns an error SVG and stack trace despite exit 0. Export failure, not a positive reference.'
    elif name in ['bd087_reg_48_near_miss','bd087_reg_50_near_miss']:reason='CommandArrow/ExoArrowRight participant identity class [%pLN_.@]+ includes @; suffix @@ lies in unrestricted message text. It is another valid identity, not malformed command punctuation.'
    elif name=='bd087_reg_64_near_miss':reason='CommandNewpage explicitly accepts optional @ and free text after colon. @@ is legal title content.'
    elif 'divider_NotAColour' in name:reason='CommandDivider creates Display from its label and has no command color slot. CommandCreoleColorChange catches NoSuchColorException during rendering and ignores the requested color.'
    elif 'group_secondary_NotAColour' in name:reason='CommandGrouping accepts its first color only immediately after keyword. In alt #Wheat #NotAColourAtAll Walnut, #Wheat is the second color, and #NotAColourAtAll Walnut is COMMENT text, not another color capture.'
    elif 'ref_ignored_back_NotAColour' in name:reason='CommandReferenceMultilinesOverSeveral#execute applies REF[0] only and deliberately never applies trailing BACKCOLOR.'
    elif name.startswith('bd087_size_'):reason='These sources contain participant declarations and prior messages. CommandArrow protectSize(2000) rejects the long relation; ClassDiagramFactory cannot accept the sequence-only declarations. All factories fail. Arrow-only companion sources fall through to CLASS; no contradiction after conditioning on source shape.'
    elif 'gradient_participant_' in name:reason='CommandParticipant#executeArg sends complete COLOR to HColorSet#getColor, not Colors. Composite channel/style strings are not HColor literals, even though ColorParser.exp1 admits their syntax.'
    elif 'gradient_note_' in name:reason='The final colon is immediately adjacent to the color. ColorParser PART2 negative lookahead forbids this terminator, so simple COLOR_REGEXP fallback captures #back/#shadowing/#text and HColorSet rejects it; #line.dotted has no complete simple fallback and is syntax-invalid.'
    elif name=='bd087_s_bare_footbox':reason='First registration CommandFootbox accepts optional TYPE but executeArg calls arg.get(TYPE,0).equalsIgnoreCase(show) without null handling. Bare footbox triggers NPE and eventual error diagram; CommandFootboxOld is never selected.'
    elif 'arrow_style_' in name:reason='CommandArrow LINE_STYLE grammar admits plain/norank, but applyStyle recognizes only dashed/bold/dotted/hidden; all other tokens are passed to HColorSet and these names fail.'
    elif name=='bd087_s_skinparam_next_line_brace':reason='CommandSkinParamMultilines extends CommandMultilinesBracket and requires { on the starting line. It does not inherit the different CommandMultilines2 syntaxWithFinalBracket transformation.'
    elif name=='bd087_s_sprite_inline_colour':reason='SpriteColorBuilder4096 expects palette-coded colour sprite data; arbitrary F00 is not a valid encoded colour sprite. Exact Java error retained.'
    elif 'reg_23_' in name:reason='CommandFactorySprite compressed z body 0 is not a valid encoded deflate sprite. Supplement uses uncompressed inline data to obtain valid execution.'
    elif 'reg_25_' in name:reason='CommandSpriteStdlib attempts to read the named archimate exported sprite; this resource name is unavailable. Supplement uses an existing cloudinsight export.'
    elif 'reg_28_' in name:reason='CommandSpriteFile syntax is valid but the deliberately absent private file cannot be read. Supplement uses source-pinned internal jar sprites with no external assets.'
    elif 'reg_30_' in name:reason='CommandStyleImport syntax is valid but the private stylesheet is absent. Supplement uses bundled plantuml.skin/rose.skin.'
    elif name=='bd087_attribute_url_quot':reason='UrlBuilder URL grammar excludes an unquoted interior double quote; this is source syntax, not an attribute serializer input.'
    else:reason='Unclassified disagreement; consult preserved raw reference and source.'
    disagreements.append(dict(name=name,intended_valid=r['intended_valid'],observed_valid=r['observed_valid'],java_exit=r['java_exit'],source_sha256=r['source_sha256'],reason=reason,stderr=r['java_stderr']))
(C/'intention-disagreements.json').write_text(json.dumps(disagreements,indent=2)+'\n')
audit=json.loads((C/'source-audit.json').read_text());mapping=[]
for item in audit['registry']:
    probes=[r for r in after if r.get('registration_index')==item['index']]
    mapping.append(dict(item,probes=[dict(name=r['name'],source=r['source'],java_valid=r['observed_valid'],java_exit=r['java_exit'],cli_exit=r['cli']['exit'],cli_diagnostic=r['cli']['stderr'],whole_output=r['cli_comparison']['exit']) for r in probes]))
(C/'handler-review.json').write_text(json.dumps(mapping,indent=2)+'\n')
print(json.dumps(result,indent=2))
