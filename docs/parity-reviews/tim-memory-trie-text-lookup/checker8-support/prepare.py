# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Immutable source-guided checker inputs; no reference execution."""
import base64
import hashlib
import json
from pathlib import Path

S = Path('/Users/marcelo/Library/Caches/rustuml-tim8-checker')
R = Path('/Users/marcelo/.t3/worktrees/rustuml/t3-6ec78fcf')
N = S / 'new'
N.mkdir(exist_ok=True)
mem = 'tim-memory-trie-text-lookup'
bind = 'tim-duplicate-formal-binding'
sub = 'tim-empty-sub-iterator-termination'
cases = [
 ('terminal_history', mem, ['removed-leaf versus retained-internal-node', 'prefix length and reinsertion'], 'intended_valid', '''!a="one"
!ab="two"
!abcd="four"
!undef abcd
!undef ab
class "a / ab / abc / abcd / abX" as History
!ab="again"
class "a / ab / abc / abcd / abX" as Again'''),
 ('lazy_before', mem, ['override index before first text scan', 'global fallback versus local retained prefix'], 'intended_valid', '''!ab="root"
!procedure $draw(a="short",abcd="long")
!undef abcd
class "a / ab / abc / abcd" as Lazy
!endprocedure
$draw()'''),
 ('lazy_after', mem, ['override index after first text scan', 'removal and mutation order'], 'intended_valid', '''!ab="root"
!procedure $draw(a="short",abcd="long")
!log materialize
!undef abcd
class "a / ab / abc / abcd" as Eager
!endprocedure
$draw()'''),
 ('punctuation_json', mem, ['arbitrary stored punctuation and spaces', 'longer stored key versus JSON projection'], 'intended_valid', '''!$j={"leaf":"from-object"}
!$x=%set_variable_value("$j.leaf","stored-dot")
!$x=%set_variable_value("[oak]","square")
!$x=%set_variable_value("two words","spaced")
class "$j.leaf / $j[leaf] / [oak] / two words" as KeyPaths'''),
 ('concat_original', mem, ['output markers versus original boundaries', 'function priority and value no-rescan'], 'intended_valid', '''!$a="##"
!$b="twig"
!raw="$b"
!function read()
!return "branch"
!endfunction
!$x=%set_variable_value("read()","blocked")
class "$a$b / ##$b## / raw / read() / xread() / \\n$b" as Scan'''),
 ('scope_mutations', mem, ['automatic local versus newly introduced global', 'override removal and sibling frame'], 'intended_valid', '''!procedure $leaf()
!$x=%set_variable_value("stem","global")
!endprocedure
!procedure $outer(stem="argument")
!stemmed="local-long"
$leaf()
!undef stem
class "stem / stemmed / stemmX" as Scopes
!endprocedure
$outer()'''),
 ('astral_offsets', mem, ['valid surrogate-pair trie edges', 'original boundary following shorter and longer values'], 'intended_valid', '''!$x=%set_variable_value("a😀b","pair")
!$x=%set_variable_value("a","short")
!$long="long replacement"
class "$long / a😁 / a😀b / 😀a / \\na / a²" as Offsets'''),
 ('markup_literal', mem, ['stored name inside markup', 'replacement value retained literally'], 'intended_valid', '''!$tag="label"
!literal="<$tag>"
class "<$tag> / literal / <$unknown>" as MarkupFresh'''),
 ('duplicate_typed', bind, ['duplicate named JSON and numeric-looking string', 'positional cursor across intervening formal'], 'intended_valid', '''!procedure $show(x="one",$middle=3,x="last")
!$sum=x+2
class "$sum / $middle" as TypedFresh
!endprocedure
$show(x=%str2json('"009"'),"position")'''),
 ('duplicate_positions', bind, ['duplicate formal order', 'all-positional overwrite versus captured defaults'], 'intended_valid', '''!$seed="05"
!function $join($x=$seed,$tail="end",$x=7)
!return $x+2+"/"+$tail
!endfunction
!$seed="99"
!$v=$join()+" / "+$join("one","mid",4)
class "$v / $join($x="08")" as Positions'''),
 ('duplicate_effect_order', bind, ['duplicate exact named operands evaluated once each', 'bare versus dollar keys'], 'intended_valid', '''!$n=0
!function $tick($d)
!$n=$n*10+$d
!return $d
!endfunction
!procedure $show(x=0,$x=1,x=2,$tail=3)
class "x / $x / $tail / $n" as EffectsFresh
!endprocedure
$show(x=$tick(4),x=$tick(5),$x=$tick(6),"tail")'''),
 ('duplicate_required', bind, ['required repeated formal', 'named read must not consume positional cursor'], 'intended_valid', '''!procedure $show($x,$middle,$x)
class "$x / $middle" as RequiredFresh
!endprocedure
$show($x="named","position")'''),
 ('duplicate_negative', bind, ['RPN distinct named-map cardinality', 'required formals without positional coverage'], 'intended_syntax_negative', '''!function $join($x,$x)
!return $x
!endfunction
!$v=$join($x="bad")
class Lost'''),
 ('empty_false', sub, ['empty collector before inactive condition', 'tail side-effect suppression'], 'intended_truncated_welcome', '''class Shelter
!if 0
!startsub quiet
!endsub
!endif
!$x=%intval("unreachable")
class Lost'''),
 ('empty_two', sub, ['two preceding model lines', 'comment-only body and end-tag suppression'], 'intended_truncated_valid', '''class Harbor
class Pier
!startsub quiet
/' only comment '/
!endsub
class Lost'''),
 ('blank_replay', sub, ['blank body versus empty body', 'second includesub and tail continuation'], 'intended_valid', '''!startsub spacing
 
!endsub
!includesub spacing
class ContinuedFresh'''),
 ('empty_nested', sub, ['nested includes with separate execution iterators', 'caller procedure continuation and side effects'], 'intended_valid', '''!$n=0
!procedure $run()
!include outer.iuml
!$n=$n+10
class "caller $n" as CallerFresh
!endprocedure
$run()
class "tail $n" as TailFresh'''),
 ('empty_return_include', sub, ['include stops within return function', 'return and caller effects continue'], 'intended_valid', '''!$n=0
!function $read()
!include quiet.iuml
!return $n+3
!endfunction
!$v=$read()
class "$v / $n" as ReturnFresh'''),
 ('empty_selected', sub, ['empty selected external sub versus empty collector', 'tail errors outside selected body'], 'intended_valid', '''!includesub selected.iuml!quiet
class SelectedFresh'''),
 ('empty_loop', sub, ['empty collector upstream of loop', 'zero iterations versus termination'], 'intended_truncated_welcome', '''class Beacon
!foreach $x in []
!startsub quiet
!endsub
!endfor
class Lost'''),
]
resources_text = {
 'quiet.iuml': '!$n=$n+1\n!startsub quiet\n!endsub\n!$n=999\n!$bad=%intval("unreachable")\n',
 'outer.iuml': '!include quiet.iuml\n!$n=$n+2\nclass "outer $n" as OuterFresh\n',
 'selected.iuml': '!$bad=%intval("not selected")\n!startsub quiet\n!endsub\n!$bad=%intval("not selected")\n',
}
sha = lambda b: hashlib.sha256(b).hexdigest()
rows = []
for name, mechanism, axes, intention, body in cases:
    p = N / (name + '.puml')
    assert not p.exists()
    p.write_text('@startuml\n' + body + '\n@enduml\n')
    rows.append(dict(source=str(p), mechanism=mechanism, axes=axes,
                     negative=intention == 'intended_syntax_negative', intended_validity=intention,
                     cwd='new', zone='GMT+11:00', debug='date=1774210426000,tz=AEDT+1100', env={}))
for name, value in resources_text.items():
    (N / name).write_text(value)
manifest = S / 'new-manifest.json'
manifest.write_text(json.dumps(rows, indent=2) + '\n')
probe = R / 'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java'
resources = []
for p in sorted(N.iterdir()) + [probe]:
    resources.append(dict(local_path=str(p), remote_relative_path=('new/' if p.parent == N else '') + p.name,
                          sha256=sha(p.read_bytes()), bytes=p.stat().st_size))
old = json.loads((R / 'docs/parity-reviews/tim-memory-trie-text-lookup/maker-support/remote-request.json').read_bytes())
contracts = []
for row in rows:
    p = Path(row['source'])
    c = dict(old['case_contracts'][0])
    c.update(source=str(p), source_sha256=sha(p.read_bytes()))
    c['probe_argv'] = c['probe_argv'][:-2] + ['<worker>/new/' + p.name, '<worker>/new']
    contracts.append(c)
request = dict(old, manifest_path=str(manifest), manifest_sha256=sha(manifest.read_bytes()),
               case_count=len(rows), resources=resources, case_contracts=contracts)
p = S / 'remote-request.json'
p.write_text(json.dumps(request, indent=2) + '\n')
bundle = {'manifest': rows, 'request': request, 'files': [{**r, 'base64': base64.b64encode(Path(r['local_path']).read_bytes()).decode()} for r in resources]}
(S / 'input-bundle.json').write_text(json.dumps(bundle, indent=2) + '\n')
for f in [p, manifest, S / 'input-bundle.json']:
    print(f, sha(f.read_bytes()))
