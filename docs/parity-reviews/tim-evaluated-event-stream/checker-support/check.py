# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent CLI comparison; comparator and eligibility are imported unchanged."""
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import yaml

ROOT = Path.cwd()
SCRATCH = Path('/tmp/rustuml-tim-checker-d2205b7f')
SUPPORT = Path(__file__).resolve().parent
JAVA_HOME = '/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home'
JAVA = JAVA_HOME+'/bin/java'
JAR = '/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
ENV = dict(os.environ, PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib', RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100', JAVA_HOME=JAVA_HOME, PATH=JAVA_HOME+'/bin:'+os.environ['PATH'])
BIN = SCRATCH/'target/release/rustuml'
MECHANISMS = ['tim-evaluated-event-stream','tim-procedure-global-fork-memory','tim-literal-variable-lookup','tim-explicit-date-instant']

def run(args, data=None, env=ENV):
    return subprocess.run([str(a) for a in args], input=data, capture_output=True, env=env, timeout=90)

def prepare():
    tracked = run(['git','ls-tree','-r','--name-only','bb2bdb67d5efc780de2c04f788dbbe8d53b54196','test-diagrams/perturbations']).stdout.decode().splitlines()
    inventory = [{'source':s, 'source_sha256':hashlib.sha256(Path(s).read_bytes()).hexdigest(), 'golden_sha256':hashlib.sha256(Path(s).with_suffix('.svg').read_bytes()).hexdigest()} for s in tracked if s.endswith('.puml')]
    assert len(inventory)==913
    (SUPPORT/'prepared-inventory.json').write_text(json.dumps(inventory, indent=2)+'\n')
    cases = {}
    def add(m,name,body,axes):
        path=Path('test-diagrams/perturbations')/MECHANISMS[m]/('checker_'+name+'.puml')
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text('@startuml\n'+body+'\n@enduml\n')
        cases[str(path)]={'axes':axes, 'kind':'fresh checker'}
    add(0,'dynamic_sub_sprite', '''!$serial = 4
!startsub palette
!$serial = $serial + 1
sprite $cedar [2x2/16] {
F0
0F
}
!endsub
!function $palette($key)
!return $key
!endfunction
!if 0
!theme minty
!endif
!theme $palette("cerulean")
class "Cedar $serial" as Cedar''', ['subblock mutation plus sprite pixels','dynamic theme and inactive ordering'])
    add(0,'comments_and_origins', '''
/' before '/ !theme cerulean
!if 0
!theme united
!endif
' consume this
class Larch /' trailing '/

class Pine
Larch --> Pine''', ['leading/internal blank counts and comments','inactive/active themes and renamed topology'])
    add(0,'comment_closure_tail', '''!theme cerulean
/' long comment
hidden
'/ class Juniper
class Alder''', ['long-comment closing line with content','theme plus two renamed classes'])
    add(0,'spaces_identity', '''!theme cerulean
   
\t
class Hawthorn''', ['nonempty whitespace versus blank identity','theme and renamed entity'])
    add(0,'two_dynamic_themes', '''!$first = "minty"
!$second = "cerulean"
!theme $first
!if 0
!theme united
!endif
!theme $second
class Rowan''', ['theme ordering and inactive load','computed selector variables and renamed label'])
    add(0,'loop_sprite_comment', '''!theme cerulean
!foreach $leaf in ["Cedar", "Willow", "Birch"]
class $leaf
!endfor
sprite $twig [1x2/16] {
F
A
}
note "branch ' annotation" as BranchNote''', ['loop cardinality and renaming','sprite placement and apostrophe content'])
    add(0,'return_function_theme_event', '''!$calls = 0
!function $configure()
!$calls = $calls + 1
!theme cerulean
!return "configured"
!endfunction
!$label = $configure()
class "$label $calls" as ReturnTheme''', ['theme event in return-valued function','global mutation and renamed label'])
    add(0,'selector_nested_theme_event', '''!function $choose()
!theme cerulean
!return "minty"
!endfunction
!theme $choose()
class SelectorTheme''', ['nested theme event during selector evaluation','active theme ordering and renamed entity'])
    add(0,'diagnostic_side_effect', '''!$calls = 0
!function $tick()
!$calls = $calls + 1
!return "diagnostic"
!endfunction
!log $tick()
!theme cerulean
class "Calls $calls" as LogTree''', ['diagnostic function side effect','theme identity and renamed counter'])
    add(1,'sibling_shadow_remove', '''!$canopy = "root"
!procedure $sibling()
class "sibling $canopy $private" as Sibling
!endprocedure
!procedure $branch($canopy)
!local $private = "leaf"
$sibling()
!undef $canopy
class "branch $canopy $private" as Branch
!endprocedure
$branch("parameter")
class "after $canopy $private" as After''', ['nested sibling frame and renamed parameter','undef override and fresh-local leakage'])
    add(1,'global_overtakes_auto_local', '''!procedure $create()
!global $cedar = "global"
!endprocedure
!procedure $outer()
!$cedar = "automatic"
$create()
class "now $cedar" as Now
!undef $cedar
class "again $cedar" as Again
!endprocedure
$outer()''', ['new global eclipses auto-local','nested call and removal reveals auto-local'])
    add(1,'explicit_override_survives_global', '''!procedure $publish()
!global $spruce = "published"
!endprocedure
!procedure $outer($spruce)
!local $local = "kept"
$publish()
class "$spruce $local" as Local
!endprocedure
$outer("argument")
class "$spruce $local" as Global''', ['parameter versus explicit global mutation','local leakage and renamed output'])
    add(1,'return_branch_isolation', '''!function $choose($seed)
!if $seed > 2
!return "tall"
!else
!return "short"
!endif
!endfunction
!procedure $emit($seed)
!if $seed > 0
!$height = $choose($seed)
class "$height" as Tree$seed
!endif
!endprocedure
!if 1
$emit(4)
$emit(1)
!endif
class Sentinel''', ['early return inside nested conditional','caller control state and two arguments'])
    add(1,'foreach_sibling', '''!$count = 0
!procedure $tick($label)
!$count = $count + 1
!local $secret = "private"
class "$label $count" as Tree$count
!endprocedure
!foreach $label in ["Oak", "Elm", "Yew"]
$tick($label)
!endfor
class "total $count $secret" as Total''', ['foreach caller state','global side effect and local privacy'])
    add(1,'default_capture', '''!$seed = "first"
!procedure $emit($label = $seed)
class "$label" as Captured
!endprocedure
!$seed = "second"
$emit()''', ['default captured at definition versus call','renamed global mutation and parameter binding'])
    add(2,'direct_computed_name', '''!$cedar = "green"
!$present = %variable_exists("$" + "cedar")
!$value = %get_variable_value("$" + "cedar")
class "$present / $value" as Computed''', ['literal concatenation name expression','exists and get operations with renamed variable'])
    add(2,'single_quoted_name', '''!$birch = "silver"
!$present = %variable_exists('$birch')
!$value = %get_variable_value('$birch')
class "$present / $value" as SingleQuoted''', ['single versus double quoted token','exists/get with renamed payload'])
    add(2,'legacy_prefix_collision', '''!define TOKEN amber
!$TOKEN = "violet"
!$bare = %get_variable_value("TOKEN")
!$dollar = %get_variable_value("$TOKEN")
class "$bare / $dollar" as Legacy''', ['legacy versus dollar namespace','same-name collision and renamed colors'])
    add(2,'nested_literal_scope', '''!$wood = "root"
!procedure $inner()
class "%get_variable_value("$wood") / %variable_exists("$bark")" as Inner
!endprocedure
!procedure $outer($wood)
!local $bark = "private"
$inner()
class "%get_variable_value("$wood") / %variable_exists("$bark")" as Outer
!endprocedure
$outer("branch")''', ['nested sibling scope and shadowing','literal lookup and missing local'])
    add(2,'variable_computed_name', '''!$maple = "ruby"
!$key = "$" + "maple"
!if %variable_exists($key)
class "%get_variable_value($key)" as VariableComputed
!endif''', ['name stored in variable','conditional and getter with renamed key'])
    add(2,'legacy_undef_scope', '''!define ACORN ochre
!procedure $inspect()
!local $seed = "bud"
!$present = %variable_exists("ACORN")
class "$present / %get_variable_value("$seed")" as Local
!undef ACORN
!endprocedure
$inspect()
class "%variable_exists("ACORN") / %variable_exists("$seed")" as After''', ['legacy define removal across call','local versus global existence'])
    for name,seconds,pattern in [
        ('minimum_seconds','-2147483648','yyyy-MM-dd HH:mm:ss EEE'),
        ('maximum_seconds','2147483647','yyyy-MM-dd HH:mm:ss EEE'),
        ('midnight_before','-39601','yyyy-MM-dd HH:mm:ss'),
        ('midnight_after','-39600','yyyy-MM-dd HH:mm:ss'),
        ('leap_expression','951782400 + 86400','yyyy-MM-dd HH:mm:ss'),
        ('format_zone','0','yyyy-MM-dd HH:mm:ss Z z XXX'),
        ('format_width','0','y-M-d H:m:s'),
        ('format_literal','946684800',"yyyy-MM-dd 'year''s day'")]:
        add(3,name,f'!$instant = {seconds}\n!$stamp = %date("{pattern}", $instant)\nclass "$stamp" as Clock{name.replace("_", "")}', ['signed instant/expression or boundary','format tokens/timezone at explicit instant'])
    # Every known valid maker failure remains executable, with its original source.
    m=MECHANISMS[0]
    retained=[Path('docs/parity-reviews')/m/'maker-cases/aws_activity_seed.puml']
    retained += [Path('test-diagrams/perturbations/tim-theme-post-expansion-source-identity')/(n+'.puml') for n in ['checker_repeated_reverse','maker_repeated_themes','checker_option_true','checker_line_continuation']]
    for source in retained:
        p=Path('test-diagrams/perturbations')/m/('retained_'+source.stem+'.puml')
        p.write_bytes(source.read_bytes())
        cases[str(p)]={'axes':['preserved maker failure replay','fresh pinned CLI reference'], 'kind':'retained valid failure', 'original_source':str(source)}
    reader=Path('test-diagrams/perturbations')/m/'checker_reader_renamed.puml'
    cases[str(reader)]={'axes':['four leading and two internal blanks','renamed classes and relationship'],'kind':'fresh checker reader control'}
    (SUPPORT/'case-manifest.json').write_text(json.dumps(cases,indent=2)+'\n')

def measure(source, fresh, meta):
    source=Path(source)
    relative=source.relative_to('test-diagrams/perturbations')
    out=(ROOT/'docs/parity-reviews'/relative.parts[0]/'checker-support'/'cases'/source.stem) if fresh else (SCRATCH/'isolated'/relative.with_suffix(''))
    out.mkdir(parents=True,exist_ok=True)
    raw=source.read_bytes()
    row={'source':str(source),'golden':str(source.with_suffix('.svg')),**meta}
    if fresh:
        p=run([JAVA,'-Djava.awt.headless=true','-Duser.timezone=GMT+11:00','-jar',JAR,'-pipe','-tsvg'],raw)
        source.with_suffix('.svg').write_bytes(p.stdout)
        (out/'java.stderr').write_bytes(p.stderr)
        row['java_exit']=p.returncode
        p=run([JAVA,'-Djava.awt.headless=true','-Duser.timezone=GMT+11:00','-cp',str(SCRATCH)+':'+JAR,'ReaderProbe','source',source])
        (out/'java.uml-source').write_bytes(p.stdout)
        (out/'java-source.stderr').write_bytes(p.stderr)
        row['java_source_exit']=p.returncode
    p=run([BIN,'-pipe'],raw)
    (out/'rust.svg').write_bytes(p.stdout)
    (out/'rust.stderr').write_bytes(p.stderr)
    row['cli_exit']=p.returncode
    c=run([SCRATCH/'probe','compare',source.with_suffix('.svg'),out/'rust.svg'])
    row['comparison_exit']=c.returncode
    row['result']='pass' if p.returncode==0 and c.returncode==0 else 'fail'
    (out/'comparison.txt').write_bytes(c.stdout+c.stderr)
    if fresh:
        y=run([BIN,'--yaml','-pipe'],raw)
        (out/'rust.yaml').write_bytes(y.stdout)
        (out/'rust-yaml.stderr').write_bytes(y.stderr)
        row['metadata_exit']=y.returncode
        try:
            node=yaml.compose(y.stdout)
            for key in ['diagram','meta','source']:
                node={k.value:v for k,v in node.value}[key]
            (out/'rust.uml-source').write_text(node.value)
            row['source_equal']=node.value.encode()==(out/'java.uml-source').read_bytes()
        except (KeyError,AttributeError,TypeError,yaml.YAMLError) as error:
            row['source_equal']=False
            row['metadata_error']=str(error)
    return row

if __name__=='__main__':
    suite=sys.argv[1]
    if suite=='prepare':
        prepare()
    else:
        manifest=json.loads((SUPPORT/('case-manifest.json' if suite=='fresh' else 'prepared-inventory.json')).read_text())
        items=list(manifest.items()) if suite=='fresh' else [(r['source'],{}) for r in manifest]
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
            rows=list(pool.map(lambda item:measure(item[0],suite=='fresh',item[1]),items))
        (SUPPORT/(suite+'-results.json')).write_text(json.dumps(rows,indent=2)+'\n')
        print(json.dumps({'suite':suite,'total':len(rows),'pass':sum(r['result']=='pass' for r in rows),'invalid_reference':sum(r['comparison_exit']==2 for r in rows),'cli_error':sum(r['cli_exit']!=0 for r in rows),'exact_source':sum(r.get('source_equal',False) for r in rows)},indent=2))
