#!/usr/bin/env python3
"""
Generator for PlantUML diagram type detection golden tests.

Tests all valid combinations of @start headers and no-header input to verify
how PlantUML detects (or infers) the diagram type.

Categories:
  1. Explicit @start headers with matching content (~20 cases)
  2. No @start header — raw content (10 cases)
  3. Ambiguous content under @startuml (15 cases)
  4. Named @startuml blocks (5 cases)

For each case, generates both a .puml and a .svg (rendered via picoweb,
with JAR-pipe fallback).
"""

import os
import sys
import subprocess
import zlib
import urllib.request
import urllib.error
import concurrent.futures
from pathlib import Path

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

BASE = Path(__file__).parent / "type-detection"
BASE.mkdir(parents=True, exist_ok=True)

JAR = Path.home() / "work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar"
PLANTUML_URL = os.environ.get("PLANTUML_URL", "http://127.0.0.1:8787")

# ---------------------------------------------------------------------------
# PlantUML encoding (deflate + custom base64) — same as render_all.py
# ---------------------------------------------------------------------------

PLANTUML_ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_"


def _encode6bit(b):
    return PLANTUML_ALPHABET[b & 0x3F]


def _encode3bytes(b1, b2, b3):
    c1 = b1 >> 2
    c2 = ((b1 & 0x3) << 4) | (b2 >> 4)
    c3 = ((b2 & 0xF) << 2) | (b3 >> 6)
    c4 = b3 & 0x3F
    return _encode6bit(c1) + _encode6bit(c2) + _encode6bit(c3) + _encode6bit(c4)


def plantuml_encode(text):
    """Encode PlantUML source using deflate + custom base64."""
    data = zlib.compress(text.encode("utf-8"))[2:-4]
    result = ""
    for i in range(0, len(data), 3):
        if i + 2 < len(data):
            result += _encode3bytes(data[i], data[i + 1], data[i + 2])
        elif i + 1 < len(data):
            result += _encode3bytes(data[i], data[i + 1], 0)
        else:
            result += _encode3bytes(data[i], 0, 0)
    return result


# ---------------------------------------------------------------------------
# Rendering
# ---------------------------------------------------------------------------

def render_via_server(source: str) -> bytes | None:
    """Try to render via picoweb GET endpoint. Returns SVG bytes or None."""
    encoded = plantuml_encode(source)
    url = f"{PLANTUML_URL}/svg/{encoded}"
    try:
        with urllib.request.urlopen(url, timeout=30) as resp:
            data = resp.read()
            return data if data else None
    except urllib.error.HTTPError as e:
        if e.code in (301, 302):
            loc = e.headers.get("Location", "")
            if not loc.startswith("http"):
                loc = f"{PLANTUML_URL}{loc}"
            try:
                with urllib.request.urlopen(loc, timeout=30) as resp:
                    return resp.read()
            except Exception:
                pass
        return None
    except Exception:
        return None


def render_via_jar(source: str) -> bytes | None:
    """Render via JAR pipe fallback. Returns SVG bytes or None."""
    if not JAR.exists():
        return None
    try:
        result = subprocess.run(
            ["java", "-jar", str(JAR), "-tsvg", "-pipe"],
            input=source.encode("utf-8"),
            capture_output=True,
            timeout=60,
        )
        if result.returncode == 0 and result.stdout:
            return result.stdout
        # Even on non-zero exit, PlantUML may have written error SVG to stdout
        if result.stdout:
            return result.stdout
        return None
    except Exception:
        return None


def render(source: str) -> tuple[bytes | None, str]:
    """Render source to SVG bytes. Returns (data, method) where method is
    'server', 'jar', or 'none'."""
    data = render_via_server(source)
    if data:
        return data, "server"
    data = render_via_jar(source)
    if data:
        return data, "jar"
    return None, "none"


# ---------------------------------------------------------------------------
# Case registry
# ---------------------------------------------------------------------------

cases: list[tuple[str, str]] = []  # (name, source)


def add(name: str, source: str):
    cases.append((name, source.strip() + "\n"))


# ===========================================================================
# Category 1: Explicit @start headers with matching content
# ===========================================================================

add("explicit_startuml_sequence",
"""
@startuml
Alice -> Bob : hello
Bob --> Alice : hi
@enduml
""")

add("explicit_startuml_class",
"""
@startuml
class Animal {
  +name : String
  +speak() : void
}
class Dog extends Animal {
  +fetch() : void
}
@enduml
""")

add("explicit_startuml_state",
"""
@startuml
[*] --> Idle
Idle --> Running : start
Running --> Idle : stop
Running --> [*] : finish
@enduml
""")

add("explicit_startuml_activity",
"""
@startuml
start
:Read input;
if (valid?) then (yes)
  :Process;
else (no)
  :Reject;
endif
stop
@enduml
""")

add("explicit_startuml_component",
"""
@startuml
component Frontend
component Backend
component Database
Frontend --> Backend : API calls
Backend --> Database : queries
@enduml
""")

add("explicit_startuml_usecase",
"""
@startuml
actor User
actor Admin
usecase "Login" as UC1
usecase "View Report" as UC2
usecase "Manage Users" as UC3
User --> UC1
User --> UC2
Admin --> UC3
@enduml
""")

add("explicit_startuml_deployment",
"""
@startuml
node WebServer {
  component Nginx
}
node AppServer {
  component AppService
}
database PostgreSQL
WebServer --> AppServer : HTTP
AppServer --> PostgreSQL : JDBC
@enduml
""")

add("explicit_startuml_timing",
"""
@startuml
robust "Signal" as SIG
SIG is idle
@0
SIG is active
@100
SIG is idle
@200
SIG is active
@enduml
""")

add("explicit_startuml_object",
"""
@startuml
object "Alice : Person" as alice {
  name = "Alice"
  age = 30
}
object "Bob : Person" as bob {
  name = "Bob"
  age = 25
}
alice --> bob : knows
@enduml
""")

add("explicit_startuml_nwdiag",
"""
@startuml
nwdiag {
  network internet {
    web [address = "1.2.3.4"];
  }
  network dmz {
    web;
    app;
  }
  network internal {
    app;
    db [address = "192.168.1.1"];
  }
}
@enduml
""")

add("explicit_startjson",
"""
@startjson
{
  "name": "Alice",
  "age": 30,
  "roles": ["admin", "user"],
  "address": {
    "city": "Wonderland",
    "zip": "12345"
  }
}
@endjson
""")

add("explicit_startyaml",
"""
@startyaml
name: Alice
age: 30
roles:
  - admin
  - user
address:
  city: Wonderland
  zip: "12345"
@endyaml
""")

add("explicit_startsalt",
"""
@startsalt
{
  Login    | "user@example.com"
  Password | "****"
  [Login]  | [Cancel]
}
@endsalt
""")

add("explicit_startgantt",
"""
@startgantt
Project starts 2024-01-01
[Design] lasts 5 days
[Implement] lasts 10 days
[Test] lasts 5 days
[Implement] starts at [Design]'s end
[Test] starts at [Implement]'s end
@endgantt
""")

add("explicit_startmindmap",
"""
@startmindmap
* Project
** Planning
*** Requirements
*** Timeline
** Development
*** Frontend
*** Backend
** Testing
@endmindmap
""")

add("explicit_startwbs",
"""
@startwbs
* CEO
** CTO
*** Dev Team
**** Alice
**** Bob
*** QA Team
** CFO
*** Finance
@endwbs
""")

add("explicit_startditaa",
"""
@startditaa
+--------+   +--------+
|        |   |        |
| Client +-->+ Server |
|        |   |        |
+--------+   +--------+
@endditaa
""")

add("explicit_startdot",
"""
@startdot
digraph G {
  A -> B;
  B -> C;
  A -> C;
}
@enddot
""")

add("explicit_startebnf",
"""
@startebnf
expression = term , { ( "+" | "-" ) , term } ;
term = factor , { ( "*" | "/" ) , factor } ;
factor = number | "(" , expression , ")" ;
number = digit , { digit } ;
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
@endebnf
""")

add("explicit_startregex",
r"""
@startregex
title Email address
[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}
@endregex
""")

# ===========================================================================
# Category 2: No @start header — raw content
# ===========================================================================

add("noheader_sequence",
"""
Alice -> Bob : hello
Bob --> Alice : hi there
Alice -> Bob : how are you?
""")

add("noheader_class",
"""
class Foo {
  +name : String
}
class Bar {
  +value : int
}
Foo <|-- Bar
""")

add("noheader_state",
"""
[*] --> Active
Active --> Paused : pause
Paused --> Active : resume
Active --> [*] : finish
""")

add("noheader_activity",
"""
start
:Step 1;
:Step 2;
stop
""")

add("noheader_component",
"""
component "Web" as WS
component "API" as API
component "DB" as DB
WS --> API
API --> DB
""")

add("noheader_usecase",
"""
actor User
usecase "Login" as UC1
usecase "Logout" as UC2
User --> UC1
User --> UC2
""")

add("noheader_deployment",
"""
node Server
database DB
Server --> DB : connects
""")

add("noheader_json",
"""
{
  "key": "value",
  "count": 42
}
""")

add("noheader_object",
"""
object Foo {
  name = "bar"
  count = 1
}
""")

add("noheader_note",
"""
note "This is a standalone note" as N1
note "Another note" as N2
N1 .. N2
""")

# ===========================================================================
# Category 3: Ambiguous content under @startuml
# ===========================================================================

add("ambiguous_actor_sequence",
"""
@startuml
actor Alice
actor Bob
Alice -> Bob : hello
Bob --> Alice : hi
@enduml
""")

add("ambiguous_actor_usecase",
"""
@startuml
actor User
usecase "Login" as UC1
usecase "Browse" as UC2
User --> UC1
User --> UC2
@enduml
""")

add("ambiguous_class_arrow",
"""
@startuml
class Foo
class Bar
Foo -> Bar : uses
@enduml
""")

add("ambiguous_entity_keyword",
"""
@startuml
entity Customer {
  * id : INT
  --
  name : VARCHAR(100)
}
entity Order {
  * id : INT
  --
  customer_id : INT
}
Customer ||--o{ Order : places
@enduml
""")

add("ambiguous_component_class_style",
"""
@startuml
component Frontend {
  [LoginForm]
  [Dashboard]
}
component Backend {
  [AuthService]
  [DataService]
}
[LoginForm] --> [AuthService] : calls
[Dashboard] --> [DataService] : fetches
@enduml
""")

add("ambiguous_state_arrows",
"""
@startuml
state Idle
state Running
state Stopped
Idle -> Running : start
Running -> Stopped : stop
Stopped -> Idle : reset
@enduml
""")

add("ambiguous_database_keyword",
"""
@startuml
database UserDB
database CacheDB
participant "App Server" as APP
APP -> UserDB : query
APP -> CacheDB : cache get
@enduml
""")

add("ambiguous_object_relationships",
"""
@startuml
object Order {
  id = 1
  total = 100.00
}
object LineItem {
  product = "Widget"
  qty = 2
}
Order *-- LineItem
@enduml
""")

add("ambiguous_allowmixing_class_state",
"""
@startuml
allowmixing
class Processor {
  +process()
}
state Running
state Idle
Idle --> Running : activate
Processor .. Running : drives
@enduml
""")

add("ambiguous_allowmixing_sequence_component",
"""
@startuml
allowmixing
participant Client
component "Server" as SRV
Client -> SRV : request
@enduml
""")

add("ambiguous_empty",
"""
@startuml
@enduml
""")

add("ambiguous_only_comments",
"""
@startuml
' This is a comment
' Another comment
' No actual diagram content
@enduml
""")

add("ambiguous_only_skinparams",
"""
@startuml
skinparam backgroundColor #FFFEF0
skinparam defaultFontName Arial
skinparam defaultFontSize 12
@enduml
""")

add("ambiguous_title_only",
"""
@startuml
title "My Diagram Title"
@enduml
""")

add("ambiguous_nwdiag_in_startuml",
"""
@startuml
nwdiag {
  network internet {
    web;
    mobile;
  }
  network internal {
    web;
    app;
    db;
  }
}
@enduml
""")

# ===========================================================================
# Category 4: Named @startuml blocks
# ===========================================================================

add("named_simple",
"""
@startuml my_diagram
Alice -> Bob : hello
Bob --> Alice : hi
@enduml
""")

add("named_with_spaces",
"""
@startuml "Diagram With Spaces"
class Foo
class Bar
Foo <|-- Bar
@enduml
""")

add("named_alphanumeric",
"""
@startuml diagram_123
[*] --> Active
Active --> [*]
@enduml
""")

add("named_class_content",
"""
@startuml named_classes
class Vehicle {
  +make : String
  +model : String
  +start() : void
}
class Car extends Vehicle {
  +doors : int
}
class Truck extends Vehicle {
  +payload : float
}
@enduml
""")

add("named_state_content",
"""
@startuml named_states
[*] --> Pending
Pending --> Processing : submit
Processing --> Approved : approve
Processing --> Rejected : reject
Approved --> [*]
Rejected --> [*]
@enduml
""")

# ===========================================================================
# Write .puml files and render .svg
# ===========================================================================

def write_and_render(name: str, source: str) -> tuple[str, str, str]:
    """Write .puml and render .svg. Returns (name, status, detail)."""
    puml_path = BASE / f"{name}.puml"
    svg_path = BASE / f"{name}.svg"

    puml_path.write_text(source, encoding="utf-8")

    svg_data, method = render(source)

    if svg_data:
        svg_path.write_bytes(svg_data)
        svg_text = svg_data.decode("utf-8", errors="replace")
        if "Syntax Error" in svg_text or "error" in svg_text.lower()[:500]:
            return name, "ERR", f"error SVG via {method}"
        return name, "OK", f"via {method}"
    else:
        return name, "FAIL", "server and JAR both failed"


def main():
    print(f"Output directory: {BASE}")
    print(f"Server: {PLANTUML_URL}")
    print(f"JAR: {JAR}")
    print(f"Total cases: {len(cases)}")
    print()

    # Category boundaries for reporting
    cat_boundaries = {
        "1. Explicit @start headers": (0, 20),
        "2. No @start header": (20, 30),
        "3. Ambiguous under @startuml": (30, 45),
        "4. Named @startuml blocks": (45, 50),
    }

    results: list[tuple[str, str, str]] = []

    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        futures = {pool.submit(write_and_render, name, src): (name, src)
                   for name, src in cases}
        # Collect in submission order
        ordered = {}
        for future in concurrent.futures.as_completed(futures):
            name, status, detail = future.result()
            ordered[name] = (name, status, detail)

    # Output in original order
    for name, _ in cases:
        results.append(ordered[name])

    # Print results grouped by category
    for cat_name, (start, end) in cat_boundaries.items():
        cat_results = results[start:end]
        ok = sum(1 for _, s, _ in cat_results if s == "OK")
        err = sum(1 for _, s, _ in cat_results if s == "ERR")
        fail = sum(1 for _, s, _ in cat_results if s == "FAIL")
        print(f"=== {cat_name} ({len(cat_results)} cases: OK={ok} ERR={err} FAIL={fail}) ===")
        for name, status, detail in cat_results:
            pad = "  " if status == "OK" else ""
            print(f"  {status:4s}  {name}{' — ' + detail if detail else ''}")
        print()

    total = len(results)
    ok = sum(1 for _, s, _ in results if s == "OK")
    err = sum(1 for _, s, _ in results if s == "ERR")
    fail = sum(1 for _, s, _ in results if s == "FAIL")
    print(f"TOTAL: {total}  OK: {ok}  ERR: {err}  FAIL: {fail}")

    puml_count = len(list(BASE.glob("*.puml")))
    svg_count = len(list(BASE.glob("*.svg")))
    print(f"Files written: {puml_count} .puml, {svg_count} .svg")


if __name__ == "__main__":
    main()
