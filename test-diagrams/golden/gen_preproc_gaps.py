#!/usr/bin/env python3
"""
Generator for PlantUML preprocessor golden files covering features with 0 coverage:
  1. !foreach (20 cases)
  2. Missing built-in functions (25 cases)
  3. !foreach + !function combos (5 cases)

Generates .puml files in test-diagrams/golden/preprocessing/ and renders each
via the PlantUML picoweb server (default: http://127.0.0.1:8787).

If the server is not running, only .puml files are written and a warning is
printed. If a render fails or returns an error SVG, the .puml is still saved
but the .svg is skipped.

Usage:
    python gen_preproc_gaps.py [--server URL]
"""

import os
import sys
import zlib
import urllib.request
import urllib.error
import argparse
from pathlib import Path

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

OUTPUT_DIR = Path(__file__).parent / "preprocessing"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

PLANTUML_URL = os.environ.get("PLANTUML_URL", "http://127.0.0.1:8787")

# ---------------------------------------------------------------------------
# PlantUML picoweb encoding (deflate + custom base64)
# ---------------------------------------------------------------------------

_ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_"


def _encode6bit(b: int) -> str:
    return _ALPHABET[b & 0x3F]


def _encode3bytes(b1: int, b2: int, b3: int) -> str:
    return (
        _encode6bit(b1 >> 2)
        + _encode6bit(((b1 & 0x3) << 4) | (b2 >> 4))
        + _encode6bit(((b2 & 0xF) << 2) | (b3 >> 6))
        + _encode6bit(b3 & 0x3F)
    )


def plantuml_encode(text: str) -> str:
    """Encode PlantUML source text for use in a /svg/ URL."""
    data = zlib.compress(text.encode("utf-8"))[2:-4]  # raw deflate
    result = ""
    for i in range(0, len(data), 3):
        if i + 2 < len(data):
            result += _encode3bytes(data[i], data[i + 1], data[i + 2])
        elif i + 1 < len(data):
            result += _encode3bytes(data[i], data[i + 1], 0)
        else:
            result += _encode3bytes(data[i], 0, 0)
    return result


class RenderResult:
    """Outcome of a render attempt."""

    __slots__ = ("svg", "status")

    def __init__(self, svg: str | None, status: str) -> None:
        self.svg = svg        # SVG text, or None
        self.status = status  # "ok" | "syntax_error" | "http_error:<code>" | "unreachable"


def render(source: str) -> RenderResult:
    """Render PlantUML source to SVG via the picoweb server.

    Always returns a RenderResult.  The .svg field is None when rendering
    could not produce usable output.  An error SVG (containing 'Syntax Error')
    is still returned with status "syntax_error" so callers can save it.
    """
    encoded = plantuml_encode(source)
    url = f"{PLANTUML_URL}/svg/{encoded}"
    try:
        with urllib.request.urlopen(url, timeout=30) as resp:
            data = resp.read()
    except urllib.error.HTTPError as e:
        if e.code in (301, 302):
            loc = e.headers.get("Location", "")
            if not loc.startswith("http"):
                loc = f"{PLANTUML_URL}{loc}"
            try:
                with urllib.request.urlopen(loc, timeout=30) as resp:
                    data = resp.read()
            except Exception:
                return RenderResult(None, "unreachable")
        else:
            return RenderResult(None, f"http_error:{e.code}")
    except Exception:
        return RenderResult(None, "unreachable")

    if not data:
        return RenderResult(None, "empty_response")

    svg = data.decode("utf-8", errors="replace")
    if "Syntax Error" in svg:
        return RenderResult(svg, "syntax_error")
    return RenderResult(svg, "ok")


# ---------------------------------------------------------------------------
# File writing helpers
# ---------------------------------------------------------------------------

puml_written = 0
svg_written = 0
svg_skipped = 0
svg_error = 0

# Tracking for built-in function results
func_results: dict[str, str] = {}  # function_name -> "ok" | "error" | "skip"


def write_case(name: str, source: str, func_name: str | None = None) -> None:
    """Write a .puml file and attempt to render an .svg alongside it."""
    global puml_written, svg_written, svg_skipped, svg_error

    puml_path = OUTPUT_DIR / f"{name}.puml"
    svg_path = OUTPUT_DIR / f"{name}.svg"

    puml_path.write_text(source, encoding="utf-8")
    puml_written += 1

    result = render(source)

    if result.status == "ok":
        svg_path.write_text(result.svg, encoding="utf-8")
        svg_written += 1
        if func_name:
            func_results[func_name] = "ok"
    elif result.status == "syntax_error":
        # Save the error SVG — it is still a valid oracle output
        svg_path.write_text(result.svg, encoding="utf-8")
        svg_error += 1
        if func_name:
            func_results[func_name] = "error SVG (PlantUML syntax error)"
    else:
        # unreachable / http_error / empty_response — skip SVG
        svg_skipped += 1
        if func_name:
            func_results[func_name] = f"skipped ({result.status})"


# ---------------------------------------------------------------------------
# Section 1: !foreach (20 cases)
# ---------------------------------------------------------------------------

# 1a. Basic foreach with 2 items
write_case("preproc_foreach_basic_2items", """\
@startuml
!foreach $item in ["Alice", "Bob"]
  participant $item
!endforeach
Alice -> Bob : hello
@enduml
""")

# 1b. Basic foreach with 3 items
write_case("preproc_foreach_basic_3items", """\
@startuml
!foreach $item in ["Alice", "Bob", "Carol"]
  participant $item
!endforeach
Alice -> Bob : step 1
Bob -> Carol : step 2
Carol --> Alice : done
@enduml
""")

# 1c. Basic foreach with 5 items
write_case("preproc_foreach_basic_5items", """\
@startuml
!foreach $svc in ["Auth", "Gateway", "Cache", "DB", "Logger"]
  participant $svc
!endforeach
Auth -> Gateway : request
Gateway -> Cache : lookup
Cache -> DB : miss
DB --> Cache : data
Cache --> Gateway : cached
Gateway --> Auth : response
Logger --> Auth : logged
@enduml
""")

# 1d. Foreach generating participants (sequence diagram)
write_case("preproc_foreach_participants_seq", """\
@startuml
!foreach $p in ["Frontend", "Backend", "Database"]
  participant $p
!endforeach
Frontend -> Backend : API call
Backend -> Database : query
Database --> Backend : rows
Backend --> Frontend : JSON
@enduml
""")

# 1e. Foreach generating messages
write_case("preproc_foreach_messages", """\
@startuml
participant Client
participant Server

!foreach $step in ["connect", "authenticate", "request", "respond", "disconnect"]
  Client -> Server : $step
!endforeach
@enduml
""")

# 1f. Foreach generating classes
write_case("preproc_foreach_classes", """\
@startuml
!foreach $cls in ["UserService", "OrderService", "PaymentService"]
class $cls {
  +execute()
}
!endforeach
UserService --> OrderService
OrderService --> PaymentService
@enduml
""")

# 1g. Foreach generating states
write_case("preproc_foreach_states", """\
@startuml
[*] --> Idle

!foreach $state in ["Idle", "Processing", "Done"]
  $state : state $state
!endforeach

Idle --> Processing : start
Processing --> Done : finish
Done --> [*]
@enduml
""")

# 1h. Foreach with variable substitution in labels
write_case("preproc_foreach_label_substitution", """\
@startuml
participant Client
participant Server

!foreach $n in ["1", "2", "3"]
  Client -> Server : request $n
  Server --> Client : response $n
!endforeach
@enduml
""")

# 1i. Foreach with nested content (note inside loop)
write_case("preproc_foreach_nested_note", """\
@startuml
participant Alice
participant Bob

!foreach $msg in ["ping", "pong"]
  Alice -> Bob : $msg
  note right
    message: $msg
  end note
!endforeach
@enduml
""")

# 1j. Foreach with nested content (group inside loop)
write_case("preproc_foreach_nested_group", """\
@startuml
participant A
participant B

!foreach $op in ["read", "write"]
  group $op operation
    A -> B : $op request
    B --> A : $op response
  end
!endforeach
@enduml
""")

# 1k. Foreach combined with !define (define used inside loop)
write_case("preproc_foreach_with_define", """\
@startuml
!define ARROW ->

participant Sender
participant Receiver

!foreach $msg in ["hello", "world", "bye"]
  Sender ARROW Receiver : $msg
!endforeach
@enduml
""")

# 1l. Foreach combined with !if (conditional inside loop body)
write_case("preproc_foreach_with_if", """\
@startuml
participant A
participant B
participant C

!foreach $dest in ["B", "C"]
  !if ($dest == "B")
    A -> B : to B
  !else
    A -> C : to C
  !endif
!endforeach
@enduml
""")

# 1m. Foreach over single item
write_case("preproc_foreach_single_item", """\
@startuml
!foreach $x in ["OnlyOne"]
  participant $x
!endforeach
OnlyOne -> OnlyOne : self call
@enduml
""")

# 1n. Foreach with integer-like string values
write_case("preproc_foreach_numeric_strings", """\
@startuml
participant Master

!foreach $i in ["1", "2", "3", "4", "5"]
  participant "Worker$i" as W$i
!endforeach

Master -> W1 : task 1
Master -> W2 : task 2
Master -> W3 : task 3
Master -> W4 : task 4
Master -> W5 : task 5
@enduml
""")

# 1o. Foreach building class hierarchy
write_case("preproc_foreach_class_hierarchy", """\
@startuml
abstract class Animal

!foreach $animal in ["Dog", "Cat", "Bird"]
class $animal extends Animal {
  +speak()
}
!endforeach
@enduml
""")

# 1p. Foreach in activity diagram (actions)
write_case("preproc_foreach_activity", """\
@startuml
start

!foreach $step in ["Validate", "Process", "Store", "Notify"]
  :$step;
!endforeach

stop
@enduml
""")

# 1q. Foreach with skinparam inside loop
write_case("preproc_foreach_skinparam", """\
@startuml
!foreach $color in ["Red", "Green", "Blue"]
  skinparam participant$color {
    BackgroundColor $color
  }
!endforeach

participant A
participant B
A -> B : colored
@enduml
""")

# 1r. Nested foreach (outer/inner)
write_case("preproc_foreach_nested", """\
@startuml
!foreach $client in ["ClientA", "ClientB"]
  !foreach $server in ["Server1", "Server2"]
    $client -> $server : connect
  !endforeach
!endforeach
@enduml
""")

# 1s. Foreach generating components
write_case("preproc_foreach_components", """\
@startuml
!foreach $comp in ["UI", "API", "DB"]
  component [$comp]
!endforeach

[UI] --> [API]
[API] --> [DB]
@enduml
""")

# 1t. Foreach combined with !while (foreach provides list, while does computation)
write_case("preproc_foreach_and_while", """\
@startuml
!foreach $svc in ["AuthService", "UserService"]
  class $svc {
    !$i = 0
    !while $i < 3
      +method$i()
      !$i = $i + 1
    !endwhile
  }
!endforeach
@enduml
""")

# ---------------------------------------------------------------------------
# Section 2: Missing built-in functions (25 cases, ~2 each)
# ---------------------------------------------------------------------------

# 2a. %get_variable_value — case 1: read a defined variable
write_case("preproc_builtin_get_variable_value_defined", """\
@startuml
!$myVar = "hello"
!$retrieved = %get_variable_value("myVar")
participant Alice
note over Alice : $retrieved
@enduml
""", func_name="%get_variable_value")

# 2b. %get_variable_value — case 2: fallback when undefined
write_case("preproc_builtin_get_variable_value_undefined", """\
@startuml
!$val = %get_variable_value("noSuchVar")
participant Alice
note over Alice : $val
@enduml
""", func_name="%get_variable_value")

# 2c. %set_variable_value — case 1: set then use
write_case("preproc_builtin_set_variable_value_basic", """\
@startuml
!%set_variable_value("dynVar", "world")
!$retrieved = %get_variable_value("dynVar")
participant Alice
note over Alice : $retrieved
@enduml
""", func_name="%set_variable_value")

# 2d. %set_variable_value — case 2: overwrite existing
write_case("preproc_builtin_set_variable_value_overwrite", """\
@startuml
!$x = "first"
!%set_variable_value("x", "second")
!$x2 = %get_variable_value("x")
participant A
note over A : $x2
@enduml
""", func_name="%set_variable_value")

# 2e. %feature_exists — case 1: check a known feature
write_case("preproc_builtin_feature_exists_known", """\
@startuml
!if %feature_exists("preprocessing")
  note over A : preprocessing supported
!else
  note over A : not supported
!endif
participant A
@enduml
""", func_name="%feature_exists")

# 2f. %feature_exists — case 2: check an unknown feature
write_case("preproc_builtin_feature_exists_unknown", """\
@startuml
participant A
!if %feature_exists("nonexistent_feature_xyz")
  note over A : exists
!else
  note over A : does not exist
!endif
@enduml
""", func_name="%feature_exists")

# 2g. %file_exists — case 1: check a file that likely exists
write_case("preproc_builtin_file_exists_exists", """\
@startuml
!if %file_exists("/etc/hosts")
  note over A : file found
!else
  note over A : file not found
!endif
participant A
@enduml
""", func_name="%file_exists")

# 2h. %file_exists — case 2: check a file that doesn't exist
write_case("preproc_builtin_file_exists_missing", """\
@startuml
participant A
!if %file_exists("/no/such/file/xyz.puml")
  note over A : exists
!else
  note over A : missing
!endif
@enduml
""", func_name="%file_exists")

# 2i. %function_exists — case 1: check a user-defined function that exists
write_case("preproc_builtin_function_exists_defined", """\
@startuml
!function $myFunc()
  !return "hi"
!endfunction

participant A
!if %function_exists("myFunc")
  note over A : function exists
!else
  note over A : not found
!endif
@enduml
""", func_name="%function_exists")

# 2j. %function_exists — case 2: check a function that doesn't exist
write_case("preproc_builtin_function_exists_undefined", """\
@startuml
participant A
!if %function_exists("noSuchFunc")
  note over A : exists
!else
  note over A : not found
!endif
@enduml
""", func_name="%function_exists")

# 2k. %invoke_procedure — case 1: invoke a defined procedure
write_case("preproc_builtin_invoke_procedure_basic", """\
@startuml
!procedure $drawNote($label)
  note over A : $label
!endprocedure

participant A
!%invoke_procedure("drawNote", "invoked!")
@enduml
""", func_name="%invoke_procedure")

# 2l. %invoke_procedure — case 2: invoke with multiple args
write_case("preproc_builtin_invoke_procedure_multiarg", """\
@startuml
!procedure $connect($src, $dst, $label)
  $src -> $dst : $label
!endprocedure

participant X
participant Y
!%invoke_procedure("connect", "X", "Y", "hello via invoke")
@enduml
""", func_name="%invoke_procedure")

# 2m. %darken — case 1: darken a named color
write_case("preproc_builtin_darken_named", """\
@startuml
!$dark = %darken("red", 30)
skinparam participantBackgroundColor $dark
participant Alice
participant Bob
Alice -> Bob : darkened
@enduml
""", func_name="%darken")

# 2n. %darken — case 2: darken a hex color
write_case("preproc_builtin_darken_hex", """\
@startuml
!$d = %darken("#4488FF", 20)
skinparam participantBackgroundColor $d
participant X
X -> X : dark hex
@enduml
""", func_name="%darken")

# 2o. %lighten — case 1: lighten a named color
write_case("preproc_builtin_lighten_named", """\
@startuml
!$light = %lighten("blue", 40)
skinparam participantBackgroundColor $light
participant Alice
participant Bob
Alice -> Bob : lightened
@enduml
""", func_name="%lighten")

# 2p. %lighten — case 2: lighten a hex color
write_case("preproc_builtin_lighten_hex", """\
@startuml
!$l = %lighten("#220044", 50)
skinparam participantBackgroundColor $l
participant X
X -> X : light hex
@enduml
""", func_name="%lighten")

# 2q. %hsl_color — case 1: pure hue
write_case("preproc_builtin_hsl_color_basic", """\
@startuml
!$c = %hsl_color(120, 100, 50)
skinparam participantBackgroundColor $c
participant Green
Green -> Green : HSL green
@enduml
""", func_name="%hsl_color")

# 2r. %hsl_color — case 2: muted tone
write_case("preproc_builtin_hsl_color_muted", """\
@startuml
!$c = %hsl_color(30, 60, 70)
skinparam participantBackgroundColor $c
participant Warm
Warm -> Warm : warm muted
@enduml
""", func_name="%hsl_color")

# 2s. %is_dark — case 1: dark color returns true
write_case("preproc_builtin_is_dark_true", """\
@startuml
participant A
!if %is_dark("#000000")
  note over A : is dark
!else
  note over A : not dark
!endif
@enduml
""", func_name="%is_dark")

# 2t. %is_dark — case 2: light color returns false
write_case("preproc_builtin_is_dark_false", """\
@startuml
participant A
!if %is_dark("#FFFFFF")
  note over A : is dark
!else
  note over A : not dark
!endif
@enduml
""", func_name="%is_dark")

# 2u. %is_light — case 1: light color returns true
write_case("preproc_builtin_is_light_true", """\
@startuml
participant A
!if %is_light("#FFFFFF")
  note over A : is light
!else
  note over A : not light
!endif
@enduml
""", func_name="%is_light")

# 2v. %is_light — case 2: dark color returns false
write_case("preproc_builtin_is_light_false", """\
@startuml
participant A
!if %is_light("#000000")
  note over A : is light
!else
  note over A : not light
!endif
@enduml
""", func_name="%is_light")

# 2w. %not — case 1: not(true)
write_case("preproc_builtin_not_true", """\
@startuml
participant A
!if %not(%true())
  note over A : was false
!else
  note over A : was true (not inverted correctly)
!endif
@enduml
""", func_name="%not")

# 2x. %not — case 2: not(false)
write_case("preproc_builtin_not_false", """\
@startuml
participant A
!if %not(%false())
  note over A : false negated to true
!else
  note over A : unexpected
!endif
@enduml
""", func_name="%not")

# 2y. %size — case 1: size of a JSON array
write_case("preproc_builtin_size_array", """\
@startuml
!$arr = ["a", "b", "c", "d"]
!$n = %size($arr)
participant A
note over A : size = $n
@enduml
""", func_name="%size")

# 2z. %size — case 2: size of a string
write_case("preproc_builtin_size_string", """\
@startuml
!$s = "hello"
!$n = %size($s)
participant A
note over A : length = $n
@enduml
""", func_name="%size")

# ---------------------------------------------------------------------------
# Section 3: !foreach + !function combo (5 cases)
# ---------------------------------------------------------------------------

# 3a. Function that formats an item; foreach calls it
write_case("preproc_foreach_func_format_label", """\
@startuml
!function $label($name)
  !return "[" + $name + "]"
!endfunction

participant Client
participant Server

!foreach $msg in ["login", "fetch", "logout"]
  Client -> Server : $label($msg)
!endforeach
@enduml
""")

# 3b. Function that generates a note; foreach calls it per item
write_case("preproc_foreach_func_generate_note", """\
@startuml
!procedure $annotate($who, $text)
  note over $who : $text
!endprocedure

participant Alice
participant Bob

!foreach $item in ["step one", "step two", "step three"]
  Alice -> Bob : $item
  !$annotate("Bob", $item)
!endforeach
@enduml
""")

# 3c. Function that checks a condition per item; foreach drives iteration
write_case("preproc_foreach_func_conditional", """\
@startuml
!function $classify($svc)
  !if $svc == "DB"
    !return "storage"
  !else
    !return "logic"
  !endif
!endfunction

!foreach $svc in ["API", "DB", "Cache"]
  class $svc {
    type: $classify($svc)
  }
!endforeach
@enduml
""")

# 3d. Foreach builds participant list; function generates message label
write_case("preproc_foreach_func_msg_builder", """\
@startuml
!function $req($src, $dst)
  !return $src + " calls " + $dst
!endfunction

!foreach $svc in ["Auth", "Order", "Pay"]
  participant $svc
!endforeach

Auth -> Order : $req("Auth", "Order")
Order -> Pay : $req("Order", "Pay")
Pay --> Order : $req("Pay", "Order") + " response"
Order --> Auth : $req("Order", "Auth") + " response"
@enduml
""")

# 3e. Nested: foreach inside a procedure, called twice with different context
write_case("preproc_foreach_func_proc_wrapping_foreach", """\
@startuml
!procedure $drawChain($prefix, $steps)
  !foreach $s in $steps
    :$prefix $s;
  !endforeach
!endprocedure

start
$drawChain("Phase A:", ["init", "validate", "run"])
$drawChain("Phase B:", ["cleanup", "report"])
stop
@enduml
""")

# ---------------------------------------------------------------------------
# Summary report
# ---------------------------------------------------------------------------

total_cases = puml_written
print(f"\n=== gen_preproc_gaps.py results ===")
print(f"Output dir : {OUTPUT_DIR}")
print(f"Server     : {PLANTUML_URL}")
print(f"PUML files : {puml_written}")
print(f"SVG ok     : {svg_written}")
print(f"SVG errors : {svg_error}  (saved with syntax/error content)")
print(f"SVG skipped: {svg_skipped}  (server unreachable or hard failure)")
print()

if func_results:
    print("=== Built-in function render results ===")
    max_len = max(len(k) for k in func_results)
    for fn, status in sorted(func_results.items()):
        print(f"  {fn:<{max_len}}  {status}")
    print()

    worked = sorted(set(k for k, v in func_results.items() if v == "ok"))
    syntax_errors = sorted(set(k for k, v in func_results.items() if "syntax error" in v))
    skipped = sorted(set(k for k, v in func_results.items() if v.startswith("skipped")))

    if worked:
        print(f"Worked      ({len(worked)}): {', '.join(worked)}")
    if syntax_errors:
        print(f"Syntax err  ({len(syntax_errors)}): {', '.join(syntax_errors)}")
    if skipped:
        print(f"Unsupported ({len(skipped)}): {', '.join(skipped)}")

if svg_skipped == puml_written:
    print(
        "\nWARNING: All SVGs were skipped. Is the PlantUML server running?\n"
        f"  scripts/plantuml-server.sh &   # starts on port 8787\n"
        f"  # or: PLANTUML_URL=http://host:port python gen_preproc_gaps.py"
    )
